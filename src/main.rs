use clap;
use kube;
use log;
use std::env;

use vault2kube::runner::Runner;
use vault2kube::vault_client;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let app = clap::App::new("vault2kube")
        .version(clap::crate_version!())
        .about("Copy Vault leased secret to Kubernetes secret")
        .setting(clap::AppSettings::SubcommandRequired)
        .subcommand(
            clap::SubCommand::with_name("run").about("Run").arg(
                clap::Arg::with_name("namespace")
                    .long("namespace")
                    .short("n")
                    .takes_value(true)
                    .required(false)
                    .help("Restrict namespace to find and execute rules from"),
            ),
        );
    let matches = app.get_matches();
    run_subcommand(matches.subcommand())
}

#[tokio::main]
async fn run_subcommand(subcommand: (&str, Option<&clap::ArgMatches>)) -> anyhow::Result<()> {
    match subcommand {
        ("run", Some(run_command)) => run(run_command).await,
        _ => panic!("?"),
    }
}

async fn run(args: &clap::ArgMatches<'_>) -> anyhow::Result<()> {
    log::info!("==> Starting...");
    let kube_client = kube::Client::try_default();
    let vault_client = vault_client::Client::new();
    let namespace = args.value_of("namespace").and_then(|s| Some(s.to_string()));
    let runner = Runner::new(kube_client.await?, vault_client.await?, namespace);
    runner.run().await
}
