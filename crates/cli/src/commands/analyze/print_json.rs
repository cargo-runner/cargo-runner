//! JSON emission for `runnables --json`.

use anyhow::Result;
use cargo_runner_core::Runnable;

use crate::display::ide_json::{CommandPreview, RunnableEntry};

pub(crate) fn emit_runnables_json(
    runner: &mut cargo_runner_core::UnifiedRunner,
    runnables: Vec<Runnable>,
    as_entries: bool,
    with_commands: bool,
) -> Result<()> {
    if as_entries {
        let entries: Vec<RunnableEntry> = runnables
            .into_iter()
            .map(|runnable| {
                let command = if with_commands {
                    runner
                        .build_command_for_runnable(&runnable)
                        .ok()
                        .flatten()
                        .map(|cmd| CommandPreview::from_command(&cmd))
                } else {
                    None
                };
                RunnableEntry { runnable, command }
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&runnables)?);
    }
    Ok(())
}
