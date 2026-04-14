use crate::agent::loop_runner::AgentMove;
use crate::error::AppError;
use chrono::{DateTime, Local};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct VaultConfig {
    pub vault_path: PathBuf,
    pub run_number: u32,
    pub session_file: PathBuf,
    pub started_at: DateTime<Local>,
}

pub struct StepLog {
    pub step: u32,
    pub action: String,
    pub confidence: f64,
    pub reasoning: String,
    pub score: u32,
    pub is_final: bool,
}

pub struct SessionSummary {
    pub result: String, // "won" | "game_over" | "stopped"
    pub score: u32,
    pub max_tile: u32,
    pub steps: u32,
    pub duration_seconds: u64,
    pub model: String,
    pub last_10_moves: Vec<AgentMove>,
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

fn game_dir(vault: &Path, game_name: &str) -> PathBuf {
    vault.join("GameSense").join(game_name)
}

fn sessions_dir(vault: &Path, game_name: &str) -> PathBuf {
    game_dir(vault, game_name).join("sessions")
}

pub fn strategies_path(vault: &Path, game_name: &str) -> PathBuf {
    game_dir(vault, game_name).join("strategies.md")
}

fn index_path(vault: &Path, game_name: &str) -> PathBuf {
    game_dir(vault, game_name).join("index.base")
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Count .md files in sessions/ to determine the next run number (1-based).
pub fn next_run_number(vault_path: &Path, game_name: &str) -> u32 {
    let dir = sessions_dir(vault_path, game_name);
    if !dir.exists() {
        return 1;
    }
    let count = fs::read_dir(&dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|x| x == "md")
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0);
    count as u32 + 1
}

/// Read strategies.md and return its contents. Returns "" if file does not exist yet.
pub fn read_strategies(vault_path: &Path, game_name: &str) -> String {
    let path = strategies_path(vault_path, game_name);
    fs::read_to_string(&path).unwrap_or_default()
}

/// Returns true if strategies.md exists AND already contains pre-game research
/// (i.e. was written by `write_initial_research`). Prevents re-researching on
/// subsequent runs of the same game.
pub fn has_initial_research(vault_path: &Path, game_name: &str) -> bool {
    let path = strategies_path(vault_path, game_name);
    match fs::read_to_string(&path) {
        Ok(content) => content.contains("## Game Mechanics") || content.contains("pre_game_research: true"),
        Err(_) => false,
    }
}

/// Write LLM-recalled game knowledge as the initial strategies.md before any session starts.
/// Skips if the file already exists (don't overwrite earned session lessons).
pub fn write_initial_research(
    vault_path: &Path,
    game_name: &str,
    research: &str,
    date_str: &str,
) -> Result<(), AppError> {
    let path = strategies_path(vault_path, game_name);
    if path.exists() {
        return Ok(()); // never overwrite existing session data
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            AppError::new("VAULT_RESEARCH_FAILED", &format!("Cannot create dir: {}", e))
        })?;
    }
    let tag = format!("gamesense/{}", game_name.to_lowercase().replace(' ', "-"));
    let content = format!(
        "---\ntags:\n  - {tag}\ngame: {game}\npre_game_research: true\nresearched_on: {date}\nsessions_analyzed: 0\nbest_score: 0\n---\n\n\
         # {game} Knowledge Base\n\n\
         ## Game Mechanics\n\n\
         {research}\n\n\
         ---\n\n\
         ## Session Lessons\n\n\
         _(Will be filled in automatically after each session.)_\n",
        tag = tag,
        game = game_name,
        date = date_str,
        research = research.trim(),
    );
    fs::write(&path, content).map_err(|e| {
        AppError::new("VAULT_RESEARCH_FAILED", &format!("Cannot write strategies.md: {}", e))
    })
}

/// Create all necessary directories and the session file with frontmatter + header.
/// Returns a VaultConfig that must be passed to subsequent calls.
pub fn init_session(
    vault_path: &Path,
    game_name: &str,
    run_number: u32,
    model: &str,
) -> Result<VaultConfig, AppError> {
    let started_at = Local::now();
    let dir = sessions_dir(vault_path, game_name);
    fs::create_dir_all(&dir).map_err(|e| {
        AppError::new("VAULT_INIT_FAILED", &format!("Cannot create session dir: {}", e))
    })?;

    let tag = format!("gamesense/{}", game_name.to_lowercase().replace(' ', "-"));
    let filename = format!(
        "{}_run-{:03}.md",
        started_at.format("%Y-%m-%d_%H-%M"),
        run_number
    );
    let session_file = dir.join(&filename);

    let frontmatter = format!(
        "---\ntags:\n  - {tag}\ngame: {game}\ndate: {date}\nrun: {run}\nresult: pending\nscore: 0\nmax_tile: 0\nsteps: 0\nduration_seconds: 0\nmodel: {model}\n---\n\n",
        tag = tag,
        game = game_name,
        date = started_at.format("%Y-%m-%d"),
        run = run_number,
        model = model,
    );

    let header = format!(
        "# {game} Session — {dt} (Run #{run})\n\n## Summary\n_(filled in after game ends)_\n\n> [!abstract] Session Lesson\n> _(filled in by LLM after game ends)_\n\n## Step Log\n\n",
        game = game_name,
        dt = started_at.format("%Y-%m-%d %H:%M"),
        run = run_number,
    );

    let content = format!("{}{}", frontmatter, header);
    fs::write(&session_file, &content).map_err(|e| {
        AppError::new("VAULT_INIT_FAILED", &format!("Cannot write session file: {}", e))
    })?;

    Ok(VaultConfig {
        vault_path: vault_path.to_path_buf(),
        run_number,
        session_file,
        started_at,
    })
}

/// Append a single step to the session file.
pub fn log_step(cfg: &VaultConfig, step: &StepLog) -> Result<(), AppError> {
    let step_label = if step.is_final {
        format!("### Step {} ⚠️ Final Move\n", step.step)
    } else {
        format!("### Step {}\n", step.step)
    };

    let callout_type = if step.is_final { "danger" } else { "tip" };

    let block = format!(
        "{step_label}**Action:** `{action}` | **Confidence:** {conf:.2} | **Score:** {score}\n\n\
         > [!{callout}] Reasoning\n\
         > {reasoning}\n\n---\n\n",
        step_label = step_label,
        action = step.action,
        conf = step.confidence,
        score = step.score,
        callout = callout_type,
        reasoning = step.reasoning.replace('\n', "\n> "),
    );

    append_to_file(&cfg.session_file, &block)
}

/// Create index.base if it does not already exist. Idempotent.
pub fn ensure_index(vault_path: &Path, game_name: &str) -> Result<(), AppError> {
    let path = index_path(vault_path, game_name);
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            AppError::new("VAULT_INDEX_FAILED", &format!("Cannot create index dir: {}", e))
        })?;
    }
    let tag = format!("gamesense/{}", game_name.to_lowercase().replace(' ', "-"));
    let content = build_index_content(game_name, &tag);
    fs::write(&path, content).map_err(|e| {
        AppError::new("VAULT_INDEX_FAILED", &format!("Cannot write index.base: {}", e))
    })
}

/// Finalize session file: write summary block + update frontmatter.
/// LLM summarize + strategies.md update is handled separately in finalize_session_async.
pub fn finalize_session_sync(
    cfg: &VaultConfig,
    summary: &SessionSummary,
) -> Result<(), AppError> {
    let duration_min = summary.duration_seconds / 60;
    let duration_sec = summary.duration_seconds % 60;
    let result_label = match summary.result.as_str() {
        "won" => "Won 🏆",
        "game_over" => "Game Over 💀",
        _ => "Stopped ⏹️",
    };

    // Append summary block at end of step log
    let summary_block = format!(
        "## Session Complete\n\n\
         **Result:** {result} | **Score:** {score} | **Max tile:** {max_tile} | **Steps:** {steps} | **Duration:** {min}m {sec}s\n\n",
        result = result_label,
        score = summary.score,
        max_tile = summary.max_tile,
        steps = summary.steps,
        min = duration_min,
        sec = duration_sec,
    );
    append_to_file(&cfg.session_file, &summary_block)?;

    // Update frontmatter fields via string replacement
    let content = fs::read_to_string(&cfg.session_file).map_err(|e| {
        AppError::new("VAULT_FINALIZE_FAILED", &format!("Cannot read session file: {}", e))
    })?;

    let updated = content
        .replacen("result: pending", &format!("result: {}", summary.result), 1)
        .replacen("score: 0", &format!("score: {}", summary.score), 1)
        .replacen("max_tile: 0", &format!("max_tile: {}", summary.max_tile), 1)
        .replacen("steps: 0", &format!("steps: {}", summary.steps), 1)
        .replacen(
            "duration_seconds: 0",
            &format!("duration_seconds: {}", summary.duration_seconds),
            1,
        );

    // Replace summary placeholder
    let updated = updated.replacen(
        "## Summary\n_(filled in after game ends)_",
        &format!(
            "## Summary\n**Result:** {result} | **Score:** {score} | **Max tile:** {max_tile} | **Steps:** {steps} | **Duration:** {min}m {sec}s",
            result = result_label,
            score = summary.score,
            max_tile = summary.max_tile,
            steps = summary.steps,
            min = duration_min,
            sec = duration_sec,
        ),
        1,
    );

    fs::write(&cfg.session_file, updated).map_err(|e| {
        AppError::new("VAULT_FINALIZE_FAILED", &format!("Cannot write session file: {}", e))
    })
}

/// Fill the lesson callout in the session file with the LLM-generated lesson text.
pub fn write_lesson_to_session(cfg: &VaultConfig, lesson: &str) -> Result<(), AppError> {
    let content = fs::read_to_string(&cfg.session_file).map_err(|e| {
        AppError::new("VAULT_LESSON_FAILED", &format!("Cannot read session file: {}", e))
    })?;

    let updated = content.replacen(
        "> [!abstract] Session Lesson\n> _(filled in by LLM after game ends)_",
        &format!(
            "> [!abstract] Session Lesson\n> {}",
            lesson.trim().replace('\n', "\n> ")
        ),
        1,
    );

    fs::write(&cfg.session_file, updated).map_err(|e| {
        AppError::new("VAULT_LESSON_FAILED", &format!("Cannot write session file: {}", e))
    })
}

/// Append a lesson entry to strategies.md.
/// If strategies.md does not exist, create it with the full template first.
pub fn append_lesson_to_strategies(
    vault_path: &Path,
    game_name: &str,
    summary: &SessionSummary,
    lesson: &str,
    run_number: u32,
    date_str: &str,
) -> Result<(), AppError> {
    let path = strategies_path(vault_path, game_name);
    let tag = format!("gamesense/{}", game_name.to_lowercase().replace(' ', "-"));

    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AppError::new("VAULT_STRATEGIES_FAILED", &format!("Cannot create strategies dir: {}", e))
            })?;
        }
        // Create strategies.md from scratch
        let initial = format!(
            "---\ntags:\n  - {tag}\ngame: {game}\nsessions_analyzed: 1\nlast_updated: {date}\nbest_score: {score}\n---\n\n\
             # {game} Knowledge Base\n\n\
             ## Core Strategy\n\n\
             _(Builds up automatically through session lessons below.)_\n\n\
             ---\n\n\
             ## Session Lessons\n\n\
             {entry}\n",
            tag = tag,
            game = game_name,
            date = date_str,
            score = summary.score,
            entry = format_lesson_entry(summary, lesson, run_number, date_str),
        );
        return fs::write(&path, initial).map_err(|e| {
            AppError::new("VAULT_STRATEGIES_FAILED", &format!("Cannot create strategies.md: {}", e))
        });
    }

    // Append to existing file
    let entry = format!("\n{}\n", format_lesson_entry(summary, lesson, run_number, date_str));
    append_to_file(&path, &entry)?;

    // Update sessions_analyzed count and last_updated / best_score in frontmatter
    let content = fs::read_to_string(&path).map_err(|e| {
        AppError::new("VAULT_STRATEGIES_FAILED", &format!("Cannot read strategies.md: {}", e))
    })?;

    // Increment sessions_analyzed
    let updated = if let Some(line) = content.lines().find(|l| l.starts_with("sessions_analyzed:")) {
        let old_count: u32 = line
            .split(':')
            .nth(1)
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        content
            .replacen(line, &format!("sessions_analyzed: {}", old_count + 1), 1)
            .replacen(
                &format!("last_updated: {}", &content
                    .lines()
                    .find(|l| l.starts_with("last_updated:"))
                    .unwrap_or("last_updated: ")),
                &format!("last_updated: {}", date_str),
                1,
            )
    } else {
        content
    };

    // Update best_score if this session beat it
    let updated = if let Some(line) = updated.lines().find(|l| l.starts_with("best_score:")) {
        let old_best: u32 = line
            .split(':')
            .nth(1)
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        if summary.score > old_best {
            updated.replacen(line, &format!("best_score: {}", summary.score), 1)
        } else {
            updated
        }
    } else {
        updated
    };

    fs::write(&path, updated).map_err(|e| {
        AppError::new("VAULT_STRATEGIES_FAILED", &format!("Cannot write strategies.md: {}", e))
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn append_to_file(path: &Path, content: &str) -> Result<(), AppError> {
    let mut file = OpenOptions::new()
        .append(true)
        .open(path)
        .map_err(|e| AppError::new("VAULT_WRITE_FAILED", &format!("Cannot open file for append: {}", e)))?;
    file.write_all(content.as_bytes())
        .map_err(|e| AppError::new("VAULT_WRITE_FAILED", &format!("Cannot write to file: {}", e)))
}

fn format_lesson_entry(
    summary: &SessionSummary,
    lesson: &str,
    run_number: u32,
    date_str: &str,
) -> String {
    let emoji = match summary.result.as_str() {
        "won" => "🏆",
        "game_over" => "💀",
        _ => "⏹️",
    };
    format!(
        "### {} Run #{} (Score: {} {})\n{}\n",
        date_str, run_number, summary.score, emoji, lesson.trim()
    )
}

// ---------------------------------------------------------------------------
// Summarize prompt builder (text-only, no image)
// ---------------------------------------------------------------------------

pub fn build_summarize_prompt(summary: &SessionSummary, existing_strategies: &str) -> String {
    let moves_text: String = summary
        .last_10_moves
        .iter()
        .map(|m| format!("  Step {}: {} (confidence {:.2}) — {}", m.step, m.action, m.confidence, m.reasoning))
        .collect::<Vec<_>>()
        .join("\n");

    let strategy_section = if existing_strategies.is_empty() {
        String::new()
    } else {
        format!(
            "\nCurrent knowledge base (do not repeat existing rules, only add new insights):\n{}\n",
            existing_strategies
        )
    };

    format!(
        "You are analyzing a completed mobile game session to extract a lesson for future games.\n\n\
         Session result: {result} | Score: {score} | Steps: {steps} | Max tile: {max_tile}\n\n\
         Last moves:\n{moves}\n\
         {strategy_section}\n\
         Write a short lesson (2-3 sentences) about what happened and one concrete rule to remember.\
         Be specific — mention move directions, tile positions, or board patterns if relevant.\n\
         Output plain text only. No markdown headers. No bullet points.",
        result = summary.result,
        score = summary.score,
        steps = summary.steps,
        max_tile = summary.max_tile,
        moves = moves_text,
        strategy_section = strategy_section,
    )
}

// ---------------------------------------------------------------------------
// index.base dynamic content
// ---------------------------------------------------------------------------

fn build_index_content(game_name: &str, tag: &str) -> String {
    format!(
        r#"filters:
  and:
    - file.inFolder("GameSense/{game}/sessions")
    - file.hasTag("{tag}")

formulas:
  efficiency: 'if(steps, (score / steps).round(0), 0)'
  result_icon: 'if(result == "won", "🏆", if(result == "game_over", "💀", "⏹️"))'
  duration_min: 'if(duration_seconds, (duration_seconds / 60).round(1), "")'

properties:
  formula.result_icon:
    displayName: ""
  formula.efficiency:
    displayName: "Score/Step"
  formula.duration_min:
    displayName: "Duration (min)"

views:
  - type: table
    name: "All Sessions"
    order:
      - file.name
      - formula.result_icon
      - score
      - max_tile
      - steps
      - formula.efficiency
      - formula.duration_min
      - model
    summaries:
      score: Max
      formula.efficiency: Average
    groupBy:
      property: result
      direction: ASC

  - type: cards
    name: "Best Runs"
    filters:
      and:
        - 'score >= 4096'
    order:
      - file.name
      - formula.result_icon
      - score
      - max_tile
"#,
        game = game_name,
        tag = tag,
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn temp_vault() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn next_run_number_returns_1_when_no_sessions() {
        let vault = temp_vault();
        assert_eq!(next_run_number(vault.path(), "2048"), 1);
    }

    #[test]
    fn init_session_creates_file_and_dirs() {
        let vault = temp_vault();
        let cfg = init_session(vault.path(), "2048", 1, "test-model").unwrap();
        assert!(cfg.session_file.exists());
        let content = fs::read_to_string(&cfg.session_file).unwrap();
        assert!(content.contains("gamesense/2048"));
        assert!(content.contains("result: pending"));
        assert!(content.contains("model: test-model"));
        assert!(content.contains("## Step Log"));
    }

    #[test]
    fn init_session_uses_game_name_in_path_and_content() {
        let vault = temp_vault();
        let cfg = init_session(vault.path(), "Subway Surfers", 1, "m").unwrap();
        assert!(cfg.session_file.exists());
        // File should be under GameSense/Subway Surfers/sessions/
        assert!(cfg.session_file.to_string_lossy().contains("Subway Surfers"));
        let content = fs::read_to_string(&cfg.session_file).unwrap();
        assert!(content.contains("game: Subway Surfers"));
        assert!(content.contains("gamesense/subway-surfers"));
    }

    #[test]
    fn next_run_number_increments_after_init() {
        let vault = temp_vault();
        init_session(vault.path(), "2048", 1, "m").unwrap();
        assert_eq!(next_run_number(vault.path(), "2048"), 2);
    }

    #[test]
    fn read_strategies_returns_empty_when_missing() {
        let vault = temp_vault();
        assert_eq!(read_strategies(vault.path(), "2048"), "");
    }

    #[test]
    fn log_step_appends_to_session_file() {
        let vault = temp_vault();
        let cfg = init_session(vault.path(), "2048", 1, "m").unwrap();
        let step = StepLog {
            step: 1,
            action: "swipe_left".to_string(),
            confidence: 0.9,
            reasoning: "Merge tiles left".to_string(),
            score: 0,
            is_final: false,
        };
        log_step(&cfg, &step).unwrap();
        let content = fs::read_to_string(&cfg.session_file).unwrap();
        assert!(content.contains("swipe_left"));
        assert!(content.contains("Merge tiles left"));
        assert!(content.contains("0.90"));
    }

    #[test]
    fn ensure_index_creates_base_file() {
        let vault = temp_vault();
        ensure_index(vault.path(), "2048").unwrap();
        let path = index_path(vault.path(), "2048");
        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("gamesense/2048"));
        assert!(content.contains("All Sessions"));
    }

    #[test]
    fn ensure_index_is_idempotent() {
        let vault = temp_vault();
        ensure_index(vault.path(), "2048").unwrap();
        ensure_index(vault.path(), "2048").unwrap(); // second call must not fail
        let path = index_path(vault.path(), "2048");
        assert!(path.exists());
    }

    #[test]
    fn ensure_index_uses_game_name() {
        let vault = temp_vault();
        ensure_index(vault.path(), "Snake").unwrap();
        let path = index_path(vault.path(), "Snake");
        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("GameSense/Snake/sessions"));
        assert!(content.contains("gamesense/snake"));
    }

    #[test]
    fn finalize_session_sync_updates_frontmatter() {
        let vault = temp_vault();
        let cfg = init_session(vault.path(), "2048", 1, "m").unwrap();
        let summary = SessionSummary {
            result: "game_over".to_string(),
            score: 1024,
            max_tile: 256,
            steps: 42,
            duration_seconds: 120,
            model: "m".to_string(),
            last_10_moves: vec![],
        };
        finalize_session_sync(&cfg, &summary).unwrap();
        let content = fs::read_to_string(&cfg.session_file).unwrap();
        assert!(content.contains("result: game_over"));
        assert!(content.contains("score: 1024"));
        assert!(content.contains("max_tile: 256"));
        assert!(content.contains("steps: 42"));
    }

    #[test]
    fn append_lesson_creates_strategies_when_missing() {
        let vault = temp_vault();
        let cfg = init_session(vault.path(), "2048", 1, "m").unwrap();
        let summary = SessionSummary {
            result: "game_over".to_string(),
            score: 512,
            max_tile: 128,
            steps: 20,
            duration_seconds: 60,
            model: "m".to_string(),
            last_10_moves: vec![],
        };
        append_lesson_to_strategies(
            vault.path(),
            "2048",
            &summary,
            "Keep tiles in corner.",
            cfg.run_number,
            "2026-04-13",
        ).unwrap();
        let path = strategies_path(vault.path(), "2048");
        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("2048 Knowledge Base"));
        assert!(content.contains("Keep tiles in corner."));
        assert!(content.contains("Session Lessons"));
    }

    #[test]
    fn append_lesson_uses_game_name_in_strategies() {
        let vault = temp_vault();
        let cfg = init_session(vault.path(), "Tetris", 1, "m").unwrap();
        let summary = SessionSummary {
            result: "game_over".to_string(),
            score: 200,
            max_tile: 0,
            steps: 10,
            duration_seconds: 30,
            model: "m".to_string(),
            last_10_moves: vec![],
        };
        append_lesson_to_strategies(
            vault.path(),
            "Tetris",
            &summary,
            "Keep the board flat.",
            cfg.run_number,
            "2026-04-13",
        ).unwrap();
        let path = strategies_path(vault.path(), "Tetris");
        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("Tetris Knowledge Base"));
        assert!(content.contains("gamesense/tetris"));
    }

    #[test]
    fn build_summarize_prompt_includes_moves_and_result() {
        use crate::agent::loop_runner::AgentMove;
        let moves = vec![AgentMove {
            step: 1,
            action: "swipe_left".to_string(),
            reasoning: "merge".to_string(),
            confidence: 0.8,
            timestamp: 0,
            score: 0,
        }];
        let summary = SessionSummary {
            result: "game_over".to_string(),
            score: 512,
            max_tile: 128,
            steps: 10,
            duration_seconds: 30,
            model: "m".to_string(),
            last_10_moves: moves,
        };
        let prompt = build_summarize_prompt(&summary, "");
        assert!(prompt.contains("game_over"));
        assert!(prompt.contains("swipe_left"));
        assert!(prompt.contains("merge"));
    }
}
