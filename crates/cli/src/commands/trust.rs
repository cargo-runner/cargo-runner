//! User-facing side of the consent gate in `cargo_runner_core::trust`.
//!
//! The core decides *whether* a command needs approval and stores decisions;
//! this module is where the human is actually asked.

use anyhow::{Context, Result};
use cargo_runner_core::{
    Command,
    trust::{self, Approval, TrustStore, Verdict},
};
use std::io::{IsTerminal, Write};

/// Ask about `command` if it needs approval, recording the answer.
///
/// Returns `Ok(())` when execution may proceed. Called before
/// `Command::execute`, which independently refuses anything unapproved — so a
/// missed call here fails safe rather than silently allowing.
pub fn ensure_trusted(command: &Command, assume_yes: bool) -> Result<()> {
    if trust::bypass_requested() {
        return Ok(());
    }

    let program = command.resolved_program();
    let Verdict::NeedsConsent(reasons) = trust::evaluate(program, &command.env) else {
        return Ok(());
    };

    let approval = trust::approval_for(command.working_dir.as_deref(), program, &command.env);
    let mut store = TrustStore::load();
    if store.contains(&approval) {
        return Ok(());
    }

    if assume_yes {
        return remember(&mut store, approval);
    }

    describe(&approval, &reasons);

    if !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "not running interactively, so this cannot be approved here.\n\
             Run `cargo runner trust` in this project to approve it, pass --trust-config, \
             or set CARGO_RUNNER_TRUST=1 in automation you control."
        );
    }

    print!("Allow this? [y]es once / [a]lways / [N]o: ");
    std::io::stdout().flush().ok();
    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .context("failed to read response")?;

    match answer.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => {
            // Allowed for this invocation only. `execute` re-checks the store,
            // so the bypass has to be made visible to this process.
            // SAFETY: single-threaded CLI startup path, before any threads spawn.
            unsafe { std::env::set_var("CARGO_RUNNER_TRUST", "1") };
            Ok(())
        }
        "a" | "always" => remember(&mut store, approval),
        _ => anyhow::bail!("not approved — nothing was run"),
    }
}

fn remember(store: &mut TrustStore, approval: Approval) -> Result<()> {
    store.insert(approval);
    store.save().context("failed to save the trust store")?;
    if let Some(p) = trust::store_path() {
        eprintln!("Recorded in {}", p.display());
    }
    // `execute` reloads the store and will now find the approval.
    Ok(())
}

fn describe(approval: &Approval, reasons: &[String]) {
    eprintln!();
    eprintln!("This project's cargo-runner config wants to run something unusual:");
    eprintln!("  project: {}", approval.root);
    for r in reasons {
        eprintln!("  · {r}");
    }
    eprintln!();
    eprintln!(
        "Approve only if you trust this repository — it is the same as running the command yourself."
    );
}

/// `cargo runner trust [--list|--revoke]` — manage stored approvals.
pub fn trust_command(list: bool, revoke: bool) -> Result<()> {
    let mut store = TrustStore::load();
    let path = trust::store_path();

    if list {
        if store.approvals.is_empty() {
            println!("No approvals recorded.");
        } else {
            for a in &store.approvals {
                println!("{}", a.root);
                println!("   program: {}", a.program);
                for (k, v) in &a.env {
                    println!("   env:     {k}={v}");
                }
            }
        }
        if let Some(p) = path {
            println!();
            println!("Store: {}", p.display());
        }
        return Ok(());
    }

    let root = std::fs::canonicalize(std::env::current_dir()?)?
        .to_string_lossy()
        .into_owned();

    if revoke {
        let before = store.approvals.len();
        store.approvals.retain(|a| a.root != root);
        let removed = before - store.approvals.len();
        store.save().context("failed to save the trust store")?;
        println!("Removed {removed} approval(s) for {root}");
        return Ok(());
    }

    println!("Approvals are recorded when you run something that needs them.");
    println!("Use --list to see them, or --revoke to drop the ones for this project.");
    println!("Current project: {root}");
    Ok(())
}
