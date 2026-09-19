use chrono::Local;
use clap::Parser;

use cronwhen::describe::describe;
use cronwhen::parse::parse_schedule;
use cronwhen::schedule::next_runs;

#[derive(Parser)]
#[command(
    name = "cronwhen",
    about = "Explains a cron expression in plain English and lists its next run times"
)]
struct Cli {
    /// A 5-field cron expression (`"*/15 * * * *"`) or `@daily`/`@hourly`-style shorthand.
    expression: String,

    /// How many upcoming run times to list.
    #[arg(short = 'n', long, default_value_t = 5)]
    count: usize,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let schedule = parse_schedule(&cli.expression)
        .map_err(|e| anyhow::anyhow!("'{}': {e}", cli.expression))?;

    println!("{}", describe(&schedule));
    println!();

    let now = Local::now();
    let runs = next_runs(&schedule, now, cli.count);
    if runs.is_empty() {
        println!(
            "no run found within the next {} years — this schedule may be unsatisfiable \
             (e.g. Feb 30) or its next occurrence is further out than this tool searches",
            5
        );
        std::process::exit(1);
    }

    println!("next {} run(s):", runs.len());
    for r in &runs {
        println!("  {}", r.format("%Y-%m-%d %H:%M:%S %a"));
    }

    Ok(())
}
