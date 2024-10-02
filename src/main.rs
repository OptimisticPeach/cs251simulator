use clap::{Parser, Subcommand};

mod simulator;
mod ui;
mod util;

use color_eyre::Report;
use simulator::{RunningState, Simulator};
use ui::setup_and_run_tui;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    specific: Option<Specific>,
}

#[derive(Subcommand)]
enum Specific {
    Run {
        #[arg(short, long)]
        file: String,

        #[arg(long, default_value_t = 1000)]
        max_iters: usize,

        #[arg(short, long)]
        out: String,
    },
    
    Load {
        #[arg(short, long)]
        file: String,
    },
}

fn main() -> Result<(), Report> {
    let args = Args::parse();

    match args.specific {
        None => setup_and_run_tui(Simulator::new())?,
        Some(Specific::Run {
            file,
            max_iters,
            out,
        }) => {
            let file = std::fs::read_to_string(&file)?;
            let mut sim = serde_json::from_str::<Simulator>(&file)?;

            for i in 0..max_iters {
                if let RunningState::ShouldStop = sim.tick()? {
                    eprintln!("Successfully exited after {i} iterations");
                    break;
                }
            }

            let to_write = serde_json::to_string_pretty(&sim)?;

            std::fs::write(out, to_write)?;
        }

        Some(Specific::Load { file }) => {
            let file = std::fs::read_to_string(&file)?;
            let sim = serde_json::from_str::<Simulator>(&file)?;

            setup_and_run_tui(sim)?;
        }
    }

    Ok(())
}
