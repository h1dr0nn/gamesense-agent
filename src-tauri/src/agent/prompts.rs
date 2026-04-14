/// Build a prompt asking the LLM to recall everything it knows about a game
/// before any session starts. The result is written to strategies.md as initial knowledge.
pub fn build_research_prompt(game_name: &str) -> String {
    format!(
        "You are a mobile game expert. I am about to play the Android game called \"{game}\".\n\n\
         Based on your training knowledge, describe this game in the following structure:\n\n\
         ## How to Play\n\
         - Core mechanic (what does the player DO each turn?)\n\
         - Win condition\n\
         - Lose/fail condition\n\n\
         ## Controls\n\
         - What gestures are used? (tap, swipe, drag?)\n\
         - How does the player interact with game elements?\n\n\
         ## Key Game Elements\n\
         - What are the main objects/pieces on screen?\n\
         - What do the visual indicators mean (colors, highlights, arrows)?\n\n\
         ## Strategy Tips\n\
         - 3-5 concrete tips for playing well\n\n\
         If you don't recognize this exact game, describe what you can infer from the name \
         and note your uncertainty. Write in plain text, no JSON. Be concise (under 300 words).",
        game = game_name,
    )
}

/// Build a game-playing prompt using AppAgent-style structured analysis.
///
/// The prompt instructs the model to:
/// 1. First inventory all visible interactive elements and their locations
/// 2. Check for tutorial indicators (hand, arrow, highlight, instruction text)
/// 3. Select the best untried action based on that analysis
///
/// `game_name` – the game being played.
/// `recent_moves` – last few moves taken this session (action strings).
/// `strategies_context` – learned strategies from previous sessions.
/// `stuck_count` – consecutive steps where the same area was tapped with no change.
/// `ui_elements` – optional list of clickable elements from UIAutomator accessibility tree
///                 (e.g. `"OK button at 50%:80% (android.widget.Button)"`). When provided,
///                 the model should prefer these precise coordinates over its own estimates.
pub fn build_game_prompt(
    game_name: &str,
    recent_moves: &[String],
    strategies_context: &str,
    stuck_count: u32,
    ui_elements: Option<&str>,
) -> String {
    let mut prompt = format!(
        "You are a game-playing AI agent. You are looking at a screenshot of \"{game}\".\n\n\
         ## GRID REFERENCE\n\
         The screenshot has a 10×10 reference grid drawn over it.\n\
         Each cell label shows: LETTER+NUMBER followed by the exact tap coordinates in percent.\n\
         Example: \"F7 55,65\" means cell F7 — tap:55%%:65%% to hit its center.\n\
         READ THE NUMBERS PRINTED ON THE GRID — do not estimate or calculate coordinates yourself.\n\
         Find the cell whose label is closest to the element you want to tap, then use those exact numbers.\n\n\
         ## STEP 1 — SCREEN ANALYSIS (do this first, every time)\n\
         Before choosing an action, list everything you observe:\n\
         a) TUTORIAL INDICATORS: Is there a hand icon, arrow, glowing highlight, or text like \
            \"Tap to...\", \"Swipe to...\", \"Touch here\"? If yes, describe exactly which grid cell it points to.\n\
         b) INTERACTIVE ELEMENTS: List all visible tappable things — buttons, columns, tiles, \
            cells, icons — with grid cell position (e.g. \"column at C5\").\n\
         c) GAME STATE: What is happening? Are you in a tutorial, main gameplay, menu, or \
            game-over screen?\n\n\
         ## STEP 2 — ACTION SELECTION\n\
         Use this priority order:\n\
         1. TUTORIAL FIRST: If a hand/arrow/highlight points at something, tap that exact element.\n\
            A \"Tap to ...\" banner means TAP — do NOT use drag or swipe regardless of past session lessons.\n\
            A \"Swipe to...\" banner means SWIPE. The on-screen instruction is ALWAYS correct. \
            Learned strategies NEVER override an explicit on-screen tutorial instruction.\n\
         2. EXPLORE UNVISITED: If no tutorial indicator, tap an interactive element you haven't \
            tried yet. Pick one far from your recent taps.\n\
         3. SYSTEMATIC GRID: If you've tried obvious elements, tap the next untried grid cell,\n\
            starting A1 → J1 → A2 → J2 → ... (row by row, left to right).\n\n\
         AVAILABLE ACTIONS:\n\
         - tap:X%:Y% — tap at position (X% from left, Y% from top)\n\
         - swipe_up / swipe_down / swipe_left / swipe_right — swipe from screen center\n\n\
         COORDINATE PRECISION RULES:\n\
         - Use the grid cells to estimate coordinates. A cell center is more reliable than free-hand guessing.\n\
         - NEVER tap the same coordinates (within 10%) more than twice in a row.\n\
         - If a tap on an element fails once, try adjacent cells (one cell = 10%).\n\
         - NEVER swipe when the tutorial says \"Tap\" — keep exploring tap positions.\n\n\
         ## RESPONSE FORMAT\n\
         Respond with a single JSON object:\n\
         {{\n\
           \"screen_analysis\": {{\n\
             \"tutorial_indicator\": \"description or null\",\n\
             \"interactive_elements\": [\"element at grid cell X%:Y%\", ...],\n\
             \"game_state\": \"tutorial | playing | menu | game_over | won\"\n\
           }},\n\
           \"reasoning\": \"why you chose this specific action and which grid cell it maps to\",\n\
           \"action\": \"tap:X%:Y% or swipe_*\",\n\
           \"confidence\": 0.0\n\
         }}\n",
        game = game_name,
    );

    if stuck_count >= 3 {
        prompt.push_str(&format!(
            "\n⚠️ STUCK ALERT: Your last {stuck} actions were in the same area with no visible change.\n\
             You MUST pick a completely different part of the screen this time.\n\
             List all elements you have NOT tapped yet and choose one of those.\n",
            stuck = stuck_count,
        ));
    }

    if let Some(ui) = ui_elements {
        prompt.push_str("\n## ACCESSIBILITY TREE (PRECISE COORDINATES)\n");
        prompt.push_str(
            "The following clickable elements were extracted from the Android accessibility tree.\n\
             Their coordinates are pixel-precise — PREFER these over your own visual estimates:\n",
        );
        for elem in ui.split(", ") {
            prompt.push_str(&format!("  • {}\n", elem));
        }
        prompt.push_str(
            "If the element you want to tap is listed above, use its exact tap:X%:Y% coordinates.\n",
        );
    }

    if !strategies_context.is_empty() {
        prompt.push_str("\n## LEARNED STRATEGIES FROM PREVIOUS SESSIONS\n");
        prompt.push_str(strategies_context);
        prompt.push('\n');
    }

    if !recent_moves.is_empty() {
        // Separate failed taps from other moves for clearer guidance
        let tap_attempts: Vec<&str> = recent_moves
            .iter()
            .filter(|m| m.starts_with("tap:"))
            .map(|m| m.as_str())
            .collect();
        let other_moves: Vec<&str> = recent_moves
            .iter()
            .filter(|m| !m.starts_with("tap:"))
            .map(|m| m.as_str())
            .collect();

        if !tap_attempts.is_empty() {
            prompt.push_str("\n## TAP POSITIONS ALREADY TRIED (do NOT repeat these)\n");
            for (i, m) in tap_attempts.iter().enumerate() {
                prompt.push_str(&format!("  {}. {}\n", i + 1, m));
            }
            prompt.push_str("  → Try positions at least 15% away from all of the above.\n");
        }
        if !other_moves.is_empty() {
            prompt.push_str("\n## OTHER RECENT MOVES\n");
            for (i, m) in other_moves.iter().enumerate() {
                prompt.push_str(&format!("  {}. {}\n", i + 1, m));
            }
        }
    }

    prompt
}

/// Count how many consecutive recent steps appear "stuck".
///
/// Stuck = the most recent action is similar to the N steps before it
/// (same action string, or tap coordinates within 10% of each other).
/// Score is ignored — meaningless for puzzle games.
pub fn count_stuck_steps(history: &[(String, u32)]) -> u32 {
    if history.len() < 2 {
        return 0;
    }

    let latest = history.last().map(|(a, _)| a.as_str()).unwrap_or("");

    let similar_count = history
        .iter()
        .rev()
        .take_while(|(a, _)| actions_similar(a, latest))
        .count() as u32;

    if similar_count >= 2 {
        similar_count
    } else {
        0
    }
}

/// Returns true if two action strings are "similar enough" to count as repeating.
/// For taps: within 10 percentage-points on both axes.
/// For swipes: exact match.
fn actions_similar(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    if let (Some(ax), Some(ay)) = parse_tap_pct(a) {
        if let (Some(bx), Some(by)) = parse_tap_pct(b) {
            return (ax - bx).abs() <= 10.0 && (ay - by).abs() <= 10.0;
        }
    }
    false
}

fn parse_tap_pct(action: &str) -> (Option<f64>, Option<f64>) {
    let s = match action.strip_prefix("tap:") {
        Some(s) => s,
        None => return (None, None),
    };
    let mut parts = s.split(':');
    let x = parts
        .next()
        .and_then(|p| p.trim_end_matches('%').parse::<f64>().ok());
    let y = parts
        .next()
        .and_then(|p| p.trim_end_matches('%').parse::<f64>().ok());
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_contains_json_format() {
        let prompt = build_game_prompt("TestGame", &[], "", 0, None);
        assert!(prompt.contains("reasoning"));
        assert!(prompt.contains("action"));
        assert!(prompt.contains("swipe_up"));
        assert!(prompt.contains("tap:X%:Y%"));
        assert!(prompt.contains("screen_analysis"));
    }

    #[test]
    fn prompt_mentions_tap_rules() {
        let prompt = build_game_prompt("TestGame", &[], "", 0, None);
        assert!(prompt.contains("Tap to ..."));
        assert!(prompt.contains("TUTORIAL FIRST"));
    }

    #[test]
    fn prompt_contains_game_name() {
        let prompt = build_game_prompt("Subway Surfers", &[], "", 0, None);
        assert!(prompt.contains("Subway Surfers"));
        assert!(!prompt.contains("2048"));
    }

    #[test]
    fn prompt_includes_tap_moves_separately() {
        let moves = vec!["tap:50%:50%".to_string(), "swipe_left".to_string()];
        let prompt = build_game_prompt("TestGame", &moves, "", 0, None);
        assert!(prompt.contains("TAP POSITIONS ALREADY TRIED"));
        assert!(prompt.contains("tap:50%:50%"));
        assert!(prompt.contains("OTHER RECENT MOVES"));
        assert!(prompt.contains("swipe_left"));
    }

    #[test]
    fn prompt_without_moves_has_no_history() {
        let prompt = build_game_prompt("TestGame", &[], "", 0, None);
        assert!(!prompt.contains("TAP POSITIONS ALREADY TRIED"));
        assert!(!prompt.contains("OTHER RECENT MOVES"));
    }

    #[test]
    fn prompt_includes_strategies_when_provided() {
        let strats = "- Prefer keeping 512 in bottom-left corner";
        let prompt = build_game_prompt("TestGame", &[], strats, 0, None);
        assert!(prompt.contains("LEARNED STRATEGIES"));
        assert!(prompt.contains("512"));
    }

    #[test]
    fn prompt_omits_strategies_section_when_empty() {
        let prompt = build_game_prompt("TestGame", &[], "", 0, None);
        assert!(!prompt.contains("LEARNED STRATEGIES"));
    }

    #[test]
    fn prompt_shows_stuck_warning_when_stuck() {
        let prompt = build_game_prompt("TestGame", &[], "", 3, None);
        assert!(prompt.contains("STUCK ALERT"));
        assert!(prompt.contains("3 actions"));
    }

    #[test]
    fn prompt_no_stuck_warning_when_not_stuck() {
        let prompt = build_game_prompt("TestGame", &[], "", 1, None);
        assert!(!prompt.contains("STUCK ALERT"));
    }

    #[test]
    fn count_stuck_repeated_same_tap() {
        let history = vec![
            ("tap:50%:50%".to_string(), 0u32),
            ("tap:50%:50%".to_string(), 0),
            ("tap:50%:50%".to_string(), 0),
        ];
        assert_eq!(count_stuck_steps(&history), 3);
    }

    #[test]
    fn count_stuck_similar_taps_within_tolerance() {
        let history = vec![
            ("tap:50%:50%".to_string(), 0u32),
            ("tap:50%:48%".to_string(), 0),
            ("tap:52%:51%".to_string(), 0),
        ];
        assert_eq!(count_stuck_steps(&history), 3);
    }

    #[test]
    fn count_stuck_different_tap_not_stuck() {
        let history = vec![
            ("tap:50%:50%".to_string(), 0u32),
            ("tap:50%:50%".to_string(), 0),
            ("tap:20%:80%".to_string(), 0),
        ];
        assert_eq!(count_stuck_steps(&history), 0);
    }

    #[test]
    fn count_stuck_score_irrelevant() {
        let history = vec![
            ("tap:50%:50%".to_string(), 0u32),
            ("tap:50%:50%".to_string(), 100),
            ("tap:50%:50%".to_string(), 200),
        ];
        assert_eq!(count_stuck_steps(&history), 3);
    }

    #[test]
    fn count_stuck_tap_only_game_no_false_positive() {
        let history = vec![
            ("tap:20%:70%".to_string(), 0u32),
            ("tap:40%:70%".to_string(), 0),
            ("tap:60%:70%".to_string(), 0),
            ("tap:80%:70%".to_string(), 0),
        ];
        assert_eq!(count_stuck_steps(&history), 0);
    }

    #[test]
    fn count_stuck_empty_history() {
        assert_eq!(count_stuck_steps(&[]), 0);
    }

    #[test]
    fn prompt_includes_grid_reference() {
        let prompt = build_game_prompt("TestGame", &[], "", 0, None);
        assert!(prompt.contains("GRID REFERENCE"));
        assert!(prompt.contains("tap coordinates"));
        assert!(prompt.contains("10×10"));
    }

    #[test]
    fn prompt_includes_ui_elements_when_provided() {
        let ui = "\"Start\" at 50%:80% (android.widget.Button)";
        let prompt = build_game_prompt("TestGame", &[], "", 0, Some(ui));
        assert!(prompt.contains("ACCESSIBILITY TREE"));
        assert!(prompt.contains("Start"));
        assert!(prompt.contains("50%:80%"));
    }

    #[test]
    fn prompt_omits_ui_elements_section_when_none() {
        let prompt = build_game_prompt("TestGame", &[], "", 0, None);
        assert!(!prompt.contains("ACCESSIBILITY TREE"));
    }
}
