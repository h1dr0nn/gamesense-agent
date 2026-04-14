use crate::agent::executor;
use crate::agent::knowledge::{self, SessionSummary, StepLog, VaultConfig};
use crate::agent::observer;
use crate::agent::prompts;
use crate::agent::providers;
use crate::agent::uiautomator;
use crate::error::AppError;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Runtime};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Observing,
    Thinking,
    Acting,
    Waiting,
    Won,
    GameOver,
    Error,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub device_id: String,
    pub api_key: String,
    pub model: String,
    pub max_steps: u32,
    pub delay_between_moves: u64,
    pub screen_width: u32,
    pub screen_height: u32,
    /// Optional base URL for OpenAI-compatible endpoints (e.g. custom OpenAI proxy, Ollama)
    pub base_url: Option<String>,
    /// Optional Obsidian vault path for logging game sessions
    pub vault_path: Option<String>,
    /// Human-readable game name used in the prompt (e.g. "Pocket Sort: Coin Puzzle")
    pub game_name: String,
    /// Package name used as vault folder key (e.g. "com.pocket.sort.coin.puzzle.game")
    pub game_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMove {
    pub step: u32,
    pub action: String,
    pub reasoning: String,
    pub confidence: f64,
    pub timestamp: u64,
    pub score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub score: u32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentStateSnapshot {
    pub status: AgentStatus,
    pub step: u32,
    pub history: Vec<AgentMove>,
    pub last_reasoning: String,
    pub game_state: Option<GameState>,
    pub error_message: Option<String>,
    /// The action being executed right now (set when status = Acting)
    pub last_action: Option<String>,
}

pub struct AgentSharedState {
    pub inner: Arc<Mutex<AgentStateInner>>,
}

pub struct AgentStateInner {
    pub status: AgentStatus,
    pub step: u32,
    pub history: Vec<AgentMove>,
    pub last_reasoning: String,
    pub game_state: Option<GameState>,
    pub error_message: Option<String>,
    pub stop_requested: bool,
    pub last_action: Option<String>,
}

impl AgentSharedState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(AgentStateInner {
                status: AgentStatus::Idle,
                step: 0,
                history: Vec::new(),
                last_reasoning: String::new(),
                game_state: None,
                error_message: None,
                stop_requested: false,
                last_action: None,
            })),
        }
    }

    pub fn snapshot(&self) -> AgentStateSnapshot {
        let inner = self.inner.lock().unwrap();
        AgentStateSnapshot {
            status: inner.status.clone(),
            step: inner.step,
            history: inner.history.clone(),
            last_reasoning: inner.last_reasoning.clone(),
            game_state: inner.game_state.clone(),
            error_message: inner.error_message.clone(),
            last_action: inner.last_action.clone(),
        }
    }
}

fn emit_state<R: Runtime>(app: &AppHandle<R>, state: &AgentSharedState) {
    let snapshot = state.snapshot();
    let _ = app.emit("agent-state-changed", &snapshot);
}

pub async fn run_agent<R: Runtime>(
    config: AgentConfig,
    state: Arc<Mutex<AgentStateInner>>,
    app: AppHandle<R>,
) {
    let shared = AgentSharedState { inner: state };
    let started_at = std::time::Instant::now();

    let vault_path = config
        .vault_path
        .as_deref()
        .map(std::path::Path::new)
        .map(std::path::Path::to_path_buf);

    let game_name = config.game_name.clone(); // human-readable label — for prompt + LLM research
    let game_id = if config.game_id.is_empty() { config.game_name.clone() } else { config.game_id.clone() }; // package id — for vault folder

    // Pre-game research: if vault is configured and no prior knowledge exists,
    // ask the LLM to recall everything it knows about this game before playing.
    if let Some(ref vp) = vault_path {
        let vp_clone = vp.clone();
        let gid = game_id.clone();
        let needs_research = tokio::task::spawn_blocking(move || {
            !knowledge::has_initial_research(&vp_clone, &gid)
        })
        .await
        .unwrap_or(false);

        if needs_research {
            eprintln!("[agent] no prior knowledge — researching game: {}", game_name);
            {
                shared.inner.lock().unwrap().status = AgentStatus::Thinking;
                emit_state(&app, &shared);
            }
            let research_prompt = prompts::build_research_prompt(&game_name);
            match providers::call_text(
                &config.api_key,
                &config.model,
                &research_prompt,
                config.base_url.as_deref(),
            )
            .await
            {
                Ok(research) => {
                    eprintln!("[agent] research complete ({} chars)", research.len());
                    let vp_clone2 = vp.clone();
                    let gid2 = game_id.clone();
                    let date_str = chrono::Local::now().format("%Y-%m-%d").to_string();
                    let _ = tokio::task::spawn_blocking(move || {
                        if let Err(e) = knowledge::write_initial_research(&vp_clone2, &gid2, &research, &date_str) {
                            eprintln!("[agent] failed to write research: {}", e);
                        }
                    })
                    .await;
                }
                Err(e) => {
                    eprintln!("[agent] research call failed (continuing without): {}", e);
                }
            }
        }
    }

    // Read strategies once at game start (includes research if just written)
    let strategies_context = if let Some(ref vp) = vault_path {
        let vp_clone = vp.clone();
        let gid = game_id.clone();
        tokio::task::spawn_blocking(move || knowledge::read_strategies(&vp_clone, &gid))
            .await
            .unwrap_or_default()
    } else {
        String::new()
    };

    // Init session (blocking file I/O)
    let vault_cfg: Option<VaultConfig> = if let Some(ref vp) = vault_path {
        let vp_clone = vp.clone();
        let gid = game_id.clone();
        let run_number = {
            let vp2 = vp_clone.clone();
            let gid2 = gid.clone();
            tokio::task::spawn_blocking(move || knowledge::next_run_number(&vp2, &gid2))
                .await
                .unwrap_or(1)
        };
        let model = config.model.clone();
        match tokio::task::spawn_blocking(move || knowledge::init_session(&vp_clone, &gid, run_number, &model)).await {
            Ok(Ok(cfg)) => {
                eprintln!("[agent] vault session: {}", cfg.session_file.display());
                Some(cfg)
            }
            Ok(Err(e)) => {
                eprintln!("[agent] failed to init session: {}", e);
                None
            }
            Err(e) => {
                eprintln!("[agent] init_session task panicked: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Ensure index file exists
    if let Some(ref vp) = vault_path {
        let vp_clone = vp.clone();
        let gid = game_id.clone();
        let _ = tokio::task::spawn_blocking(move || knowledge::ensure_index(&vp_clone, &gid)).await;
    }

    // Hard limit: stop automatically if stuck for this many consecutive steps
    const MAX_STUCK_STEPS: u32 = 15;

    loop {
        // Check stop
        {
            let inner = shared.inner.lock().unwrap();
            if inner.stop_requested || inner.step >= config.max_steps {
                break;
            }
        }

        // 1. OBSERVE
        {
            shared.inner.lock().unwrap().status = AgentStatus::Observing;
            emit_state(&app, &shared);
        }

        // Re-check stop before each expensive operation
        if shared.inner.lock().unwrap().stop_requested {
            break;
        }

        let current_step_num = shared.inner.lock().unwrap().step;
        eprintln!("[agent] step {} — taking screenshot from {}", current_step_num, config.device_id);
        let device_id = config.device_id.clone();
        // Use grid-overlay screenshot so the model can reference cell labels (A1..J10)
        let screenshot_b64 = match tokio::task::spawn_blocking(move || observer::capture_as_base64_with_grid(&device_id)).await {
            Ok(Ok(b64)) => {
                eprintln!("[agent] screenshot ok, {} bytes base64", b64.len());
                // Debug: save grid screenshot to temp dir so we can inspect what the model sees
                if let Ok(png) = base64::engine::general_purpose::STANDARD.decode(&b64) {
                    let path = std::env::temp_dir().join(format!("agent_step_{:03}.png", current_step_num));
                    let _ = std::fs::write(&path, &png);
                    eprintln!("[agent] debug screenshot saved to {}", path.display());
                }
                b64
            }
            Ok(Err(e)) => {
                eprintln!("[agent] screenshot error: {}", e);
                let mut inner = shared.inner.lock().unwrap();
                inner.status = AgentStatus::Error;
                inner.error_message = Some(format!("Screenshot failed: {}", e));
                emit_state(&app, &shared);
                break;
            }
            Err(e) => {
                eprintln!("[agent] screenshot task panic: {}", e);
                let mut inner = shared.inner.lock().unwrap();
                inner.status = AgentStatus::Error;
                inner.error_message = Some(format!("Screenshot task failed: {}", e));
                emit_state(&app, &shared);
                break;
            }
        };

        // Re-check stop before UIAutomator dump (can be slow on some devices)
        if shared.inner.lock().unwrap().stop_requested {
            break;
        }

        // UIAutomator: try to get precise clickable element coordinates from the accessibility tree.
        // This works for native Android UI; Unity/Unreal games will return an empty tree (is_useful=false).
        // Wrapped in a timeout so a slow/hanging uiautomator dump doesn't stall the loop.
        let device_id_ui = config.device_id.clone();
        let sw = config.screen_width;
        let sh = config.screen_height;
        let ui_tree = tokio::time::timeout(
            tokio::time::Duration::from_secs(3),
            tokio::task::spawn_blocking(move || {
                uiautomator::dump_ui_tree(&device_id_ui, sw, sh).ok()
            }),
        )
        .await
        .ok()
        .and_then(|r| r.ok())
        .flatten();

        let ui_hint = if let Some(ref tree) = ui_tree {
            if tree.is_useful() {
                let descs = tree.describe_clickable();
                if descs.is_empty() {
                    None
                } else {
                    eprintln!("[agent] UIAutomator found {} clickable elements", descs.len());
                    Some(descs.join(", "))
                }
            } else {
                eprintln!("[agent] UIAutomator tree not useful (likely Unity/Unreal engine)");
                None
            }
        } else {
            None
        };

        // 2. THINK
        {
            shared.inner.lock().unwrap().status = AgentStatus::Thinking;
            emit_state(&app, &shared);
        }

        let (recent_moves, stuck_count) = {
            let inner = shared.inner.lock().unwrap();
            let moves: Vec<String> = inner
                .history
                .iter()
                .rev()
                .take(5)
                .map(|m| format!("{} (confidence: {:.1})", m.action, m.confidence))
                .collect();
            let action_score_pairs: Vec<(String, u32)> = inner
                .history
                .iter()
                .map(|m| (m.action.clone(), m.score))
                .collect();
            let stuck = prompts::count_stuck_steps(&action_score_pairs);
            (moves, stuck)
        };

        // Auto-stop if stuck too long — prevents wasting all steps on tutorial loops
        if stuck_count >= MAX_STUCK_STEPS {
            eprintln!("[agent] stuck for {} steps with no score change — stopping", stuck_count);
            let mut inner = shared.inner.lock().unwrap();
            inner.status = AgentStatus::Stopped;
            inner.error_message = Some(format!(
                "Stopped: no progress after {} consecutive steps. Try a different game or check touch coordinates.",
                stuck_count
            ));
            emit_state(&app, &shared);
            break;
        }

        let prompt = prompts::build_game_prompt(
            &game_name,
            &recent_moves,
            &strategies_context,
            stuck_count,
            ui_hint.as_deref(),
        );

        // Re-check stop before the API call (which can take 2-5 seconds)
        if shared.inner.lock().unwrap().stop_requested {
            break;
        }

        eprintln!("[agent] calling vision API model={} base_url={:?}", config.model, config.base_url);
        let api_result = tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            providers::call_vision(
                &config.api_key,
                &config.model,
                &prompt,
                &screenshot_b64,
                config.base_url.as_deref(),
            ),
        )
        .await;

        // Check stop: if user stopped while we were waiting for the API, exit cleanly
        if shared.inner.lock().unwrap().stop_requested {
            break;
        }

        let response = match api_result {
            Ok(Ok(r)) => {
                eprintln!("[agent] API ok — action={} confidence={}", r.action, r.confidence);
                r
            }
            Ok(Err(e)) => {
                eprintln!("[agent] API error: {}", e);
                let mut inner = shared.inner.lock().unwrap();
                inner.status = AgentStatus::Error;
                inner.error_message = Some(format!("API failed: {}", e));
                emit_state(&app, &shared);
                break;
            }
            Err(_) => {
                eprintln!("[agent] API timeout after 30s");
                let mut inner = shared.inner.lock().unwrap();
                inner.status = AgentStatus::Error;
                inner.error_message = Some("API request timed out after 30s".to_string());
                emit_state(&app, &shared);
                break;
            }
        };

        // UIAutomator refinement: if the accessibility tree is useful AND the model's
        // action is a tap, try to find a tree element near those coordinates and use
        // its precise center instead of the model's approximate guess.
        let response = refine_tap_with_ui_tree(response, ui_tree.as_ref());

        // 3. Update game state — prefer screen_analysis.game_state, fallback to game_state.status
        let game_status = response
            .screen_analysis
            .as_ref()
            .map(|sa| sa.game_state.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(response.game_state.status.as_str())
            .to_string();

        {
            let mut inner = shared.inner.lock().unwrap();
            inner.last_reasoning = response.reasoning.clone();
            inner.game_state = Some(GameState {
                score: response.game_state.score,
                status: game_status.clone(),
            });
        }

        // 4. ACT
        {
            let mut inner = shared.inner.lock().unwrap();
            inner.status = AgentStatus::Acting;
            inner.last_action = Some(response.action.clone());
            drop(inner);
            emit_state(&app, &shared);
        }

        let current_step = shared.inner.lock().unwrap().step + 1;
        let is_terminal = matches!(game_status.as_str(), "game_over" | "won");

        // Log step to vault (non-blocking, best-effort)
        if let Some(ref cfg) = vault_cfg {
            let step_log = StepLog {
                step: current_step,
                action: response.action.clone(),
                reasoning: response.reasoning.clone(),
                confidence: response.confidence,
                score: response.game_state.score,
                is_final: is_terminal,
            };
            let cfg_clone = cfg.clone();
            let _ = tokio::task::spawn_blocking(move || {
                if let Err(e) = knowledge::log_step(&cfg_clone, &step_log) {
                    eprintln!("[agent] vault log_step failed: {}", e);
                }
            });
        }

        if game_status == "game_over" {
            shared.inner.lock().unwrap().status = AgentStatus::GameOver;
            emit_state(&app, &shared);
            break;
        }

        if game_status == "won" {
            shared.inner.lock().unwrap().status = AgentStatus::Won;
            emit_state(&app, &shared);
            break;
        }

        if let Err(e) = executor::execute_action(
            &config.device_id,
            &response.action,
            config.screen_width,
            config.screen_height,
        ) {
            let mut inner = shared.inner.lock().unwrap();
            inner.status = AgentStatus::Error;
            inner.error_message = Some(format!("Action failed: {}", e));
            emit_state(&app, &shared);
            break;
        }

        // 5. Record move
        {
            let mut inner = shared.inner.lock().unwrap();
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            inner.step += 1;
            let step = inner.step;
            let score = inner.game_state.as_ref().map(|g| g.score).unwrap_or(0);
            let agent_move = AgentMove {
                step,
                action: response.action.clone(),
                reasoning: response.reasoning.clone(),
                confidence: response.confidence,
                timestamp,
                score,
            };
            inner.history.push(agent_move);
        }

        // 6. WAIT
        {
            shared.inner.lock().unwrap().status = AgentStatus::Waiting;
            emit_state(&app, &shared);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(config.delay_between_moves)).await;

        // Check stop before waiting for screen stability — no point stabilizing if stopping
        if shared.inner.lock().unwrap().stop_requested {
            break;
        }

        let stable_device_id = config.device_id.clone();
        // Timeout at 3s so a frozen device doesn't stall the stop path
        let _ = tokio::time::timeout(
            tokio::time::Duration::from_secs(3),
            tokio::task::spawn_blocking(move || observer::wait_for_stable(&stable_device_id, 2000)),
        )
        .await;
    }

    // Final state
    let final_status = {
        let mut inner = shared.inner.lock().unwrap();
        if inner.status != AgentStatus::Won
            && inner.status != AgentStatus::GameOver
            && inner.status != AgentStatus::Error
        {
            inner.status = AgentStatus::Stopped;
        }
        inner.status.clone()
    };
    emit_state(&app, &shared);

    // Finalize vault session asynchronously
    if let (Some(vp), Some(cfg)) = (vault_path, vault_cfg) {
        let result_str = match final_status {
            AgentStatus::Won => "won",
            AgentStatus::GameOver => "game_over",
            _ => "stopped",
        }
        .to_string();

        let (final_score, max_tile) = shared
            .inner
            .lock()
            .unwrap()
            .game_state
            .as_ref()
            .map(|g| (g.score, 0u32))
            .unwrap_or((0, 0));

        let total_steps = shared.inner.lock().unwrap().step;
        let last_10_moves: Vec<AgentMove> = {
            let inner = shared.inner.lock().unwrap();
            inner.history.iter().rev().take(10).cloned().collect()
        };
        let duration_seconds = started_at.elapsed().as_secs();

        let api_key = config.api_key.clone();
        let model = config.model.clone();
        let base_url = config.base_url.clone();
        let model_for_summary = model.clone();
        let gid = game_id.clone();

        tokio::spawn(async move {
            let summary = SessionSummary {
                result: result_str,
                score: final_score,
                max_tile,
                steps: total_steps,
                duration_seconds,
                model: model_for_summary,
                last_10_moves,
            };
            // Hard cap: finalize must complete within 60s regardless of API behaviour
            let result = tokio::time::timeout(
                tokio::time::Duration::from_secs(60),
                finalize_session(&vp, &gid, cfg, summary, &strategies_context, &api_key, &model, base_url.as_deref()),
            )
            .await;
            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => eprintln!("[agent] vault finalize failed: {}", e),
                Err(_) => eprintln!("[agent] vault finalize timed out after 60s"),
            }
        });
    }
}

/// If the UIAutomator tree is useful and the agent chose a `tap:X%:Y%` action,
/// find the nearest clickable element and replace the action coordinates with
/// the element's precise pixel-center from the accessibility tree.
///
/// This corrects LLM spatial guessing (±5–15% error) with ground-truth coordinates.
/// If no element is within 20% of the tapped position, the original action is kept.
fn refine_tap_with_ui_tree(
    mut response: providers::AgentResponse,
    tree: Option<&uiautomator::UiTree>,
) -> providers::AgentResponse {
    let Some(tree) = tree else { return response };
    if !tree.is_useful() {
        return response;
    }

    // Parse the model's tap coordinates
    let Some(s) = response.action.strip_prefix("tap:") else { return response };
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return response;
    }
    let parse_pct = |p: &str| -> Option<f64> { p.trim_end_matches('%').parse().ok() };
    let (Some(tap_x), Some(tap_y)) = (parse_pct(parts[0]), parse_pct(parts[1])) else {
        return response;
    };

    // Find the closest clickable element within 20% threshold
    const MAX_DIST_PCT: f64 = 20.0;
    let closest = tree
        .elements
        .iter()
        .filter(|e| e.clickable)
        .map(|e| {
            let dx = e.center_x_pct - tap_x;
            let dy = e.center_y_pct - tap_y;
            let dist = (dx * dx + dy * dy).sqrt();
            (dist, e)
        })
        .filter(|(dist, _)| *dist <= MAX_DIST_PCT)
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    if let Some((dist, element)) = closest {
        let refined = element.tap_action();
        eprintln!(
            "[agent] UIAutomator refined tap: {} → {} (dist={:.1}%, element=\"{}\")",
            response.action, refined, dist, element.label
        );
        response.action = refined;
    }

    response
}

async fn finalize_session(
    vault_path: &std::path::Path,
    game_name: &str,
    cfg: VaultConfig,
    summary: SessionSummary,
    existing_strategies: &str,
    api_key: &str,
    model: &str,
    base_url: Option<&str>,
) -> Result<(), AppError> {
    let run_number = cfg.run_number;

    // Build prompt before moving cfg
    let prompt = knowledge::build_summarize_prompt(&summary, existing_strategies);

    // Finalize markdown (blocking)
    let cfg_clone = cfg.clone();
    let summary_ref = SessionSummary {
        result: summary.result.clone(),
        score: summary.score,
        max_tile: summary.max_tile,
        steps: summary.steps,
        duration_seconds: summary.duration_seconds,
        model: summary.model.clone(),
        last_10_moves: summary.last_10_moves.clone(),
    };
    tokio::task::spawn_blocking(move || knowledge::finalize_session_sync(&cfg_clone, &summary_ref))
        .await
        .map_err(|e| AppError::new("VAULT_TASK_FAILED", &format!("Finalize task panicked: {}", e)))??;

    // Call LLM for lesson (30s timeout — don't let a dead API block finalization)
    let lesson = match tokio::time::timeout(
        tokio::time::Duration::from_secs(30),
        providers::call_text(api_key, model, &prompt, base_url),
    )
    .await
    {
        Ok(Ok(text)) => text,
        Ok(Err(e)) => {
            eprintln!("[agent] lesson LLM call failed: {}", e);
            return Ok(()); // skip lesson, don't propagate
        }
        Err(_) => {
            eprintln!("[agent] lesson LLM call timed out");
            return Ok(());
        }
    };
    if lesson.trim().is_empty() {
        return Ok(());
    }

    // Write lesson to session + strategies (blocking)
    let cfg_clone2 = cfg.clone();
    let lesson_clone = lesson.clone();
    let vp_clone = vault_path.to_path_buf();
    let gn = game_name.to_string();
    let date_str = chrono::Local::now().format("%Y-%m-%d").to_string();
    tokio::task::spawn_blocking(move || -> Result<(), AppError> {
        knowledge::write_lesson_to_session(&cfg_clone2, &lesson_clone)?;
        knowledge::append_lesson_to_strategies(
            &vp_clone,
            &gn,
            &SessionSummary {
                result: summary.result.clone(),
                score: summary.score,
                max_tile: summary.max_tile,
                steps: summary.steps,
                duration_seconds: summary.duration_seconds,
                model: summary.model.clone(),
                last_10_moves: summary.last_10_moves.clone(),
            },
            &lesson_clone,
            run_number,
            &date_str,
        )
    })
    .await
    .map_err(|e| AppError::new("VAULT_TASK_FAILED", &format!("Write lesson task panicked: {}", e)))??;

    eprintln!("[agent] vault finalized — lesson written to strategies");
    Ok(())
}

// --- Tauri Commands ---

#[tauri::command]
pub async fn start_agent<R: Runtime>(
    config: AgentConfig,
    app: AppHandle<R>,
    state: tauri::State<'_, AgentSharedState>,
) -> Result<(), AppError> {
    // Guard: reject if a previous run is still active (not in a terminal state).
    // This prevents two concurrent agent tasks from racing on shared state.
    {
        let inner = state.inner.lock().unwrap();
        let is_active = matches!(
            inner.status,
            AgentStatus::Observing
                | AgentStatus::Thinking
                | AgentStatus::Acting
                | AgentStatus::Waiting
        );
        if is_active {
            return Err(AppError::new(
                "AGENT_ALREADY_RUNNING",
                "Agent is already running. Stop it before starting a new session.",
            ));
        }
    }

    // Reset state
    {
        let mut inner = state.inner.lock().unwrap();
        inner.status = AgentStatus::Observing;
        inner.step = 0;
        inner.history.clear();
        inner.last_reasoning.clear();
        inner.game_state = None;
        inner.error_message = None;
        inner.stop_requested = false;
    }

    let state_arc = state.inner.clone();
    let app_clone = app.clone();

    tokio::spawn(async move {
        run_agent(config, state_arc, app_clone).await;
    });

    Ok(())
}

#[tauri::command]
pub fn stop_agent(state: tauri::State<'_, AgentSharedState>) -> Result<(), AppError> {
    let mut inner = state.inner.lock().unwrap();
    inner.stop_requested = true;
    Ok(())
}

#[tauri::command]
pub fn get_agent_state(
    state: tauri::State<'_, AgentSharedState>,
) -> Result<AgentStateSnapshot, AppError> {
    Ok(state.snapshot())
}
