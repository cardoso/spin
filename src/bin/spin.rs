use anyhow::Result;
use async_trait::async_trait;
use clap::{Parser, Subcommand, CommandFactory};
use is_terminal::IsTerminal;
use lazy_static::lazy_static;
use spin_cli::{commands::{
    bindle::BindleCommands,
    build::BuildCommand,
    deploy::DeployCommand,
    login::LoginCommand,
    new::{AddCommand, NewCommand},
    oci::OciCommands,
    plugins::PluginCommands,
    templates::TemplateCommands,
    up::UpCommand, external::ExternalCommands,
}, dispatch::Action};
use spin_cli::dispatch::{Dispatch};
use anyhow::anyhow;
use spin_cli::*;
use spin_http::HttpTrigger;
use spin_redis_engine::RedisTrigger;
use spin_trigger::cli::help::HelpArgsOnlyTrigger;
use spin_trigger::cli::TriggerExecutorCommand;

#[cfg(feature = "generate-completions")]
use spin_cli::commands::generate_completions::GenerateCompletionsCommands;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(std::io::stderr().is_terminal())
        .init();
    SpinApp::parse().dispatch(&Action::Help).await?;

    Ok(())
}

lazy_static! {
    pub static ref VERSION: String = build_info();
}

/// Helper for passing VERSION to structopt.
fn version() -> &'static str {
    &VERSION
}

/// The Spin CLI
#[derive(Parser)]
#[command(id = "spin", version = version())]
enum SpinApp {
    #[command(subcommand, alias = "template")]
    Templates(TemplateCommands),
    New(NewCommand),
    Add(AddCommand),
    Up(UpCommand),
    #[command(subcommand)]
    Bindle(BindleCommands),
    #[command(subcommand)]
    Oci(OciCommands),
    Deploy(DeployCommand),
    Build(BuildCommand),
    Login(LoginCommand),
    #[command(subcommand, alias = "plugin")]
    Plugins(PluginCommands),
    #[cfg(feature = "generate-completions")]
    /// Generate shell completions
    #[command(subcommand, hide = true)]
    GenerateCompletions(GenerateCompletionsCommands),
    #[command(subcommand, hide = true)]
    Trigger(TriggerCommands),
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(Subcommand)]
enum TriggerCommands {
    Http(TriggerExecutorCommand<HttpTrigger>),
    Redis(TriggerExecutorCommand<RedisTrigger>),
    #[clap(name = spin_cli::HELP_ARGS_ONLY_TRIGGER_TYPE, hide = true)]
    HelpArgsOnly(TriggerExecutorCommand<HelpArgsOnlyTrigger>),
}

impl_dispatch!(TriggerCommands::{Http, Redis, HelpArgsOnly});

struct External;
impl Dispatch for External {

}

#[async_trait(?Send)]
impl Dispatch for SpinApp {
    /// The main entry point to Spin.
    async fn dispatch(&self, action: &Action) -> Result<()> {
        macro_rules! dispatch {
            ($cmd:expr) => {
                ($cmd).dispatch(action).await
            };
        }

        macro_rules! external {
            ($cmd:expr) => {
                {
                    let cmd: &Vec<String> = $cmd;
                    let cmd = ExternalCommands::new(cmd.to_vec(), Self::command());
                    dispatch!(cmd)
                }
            }
        }

        macro_rules! delegate {
            ($($variant:ident),*) => {
                $(if let Self::$variant(cmd) = self {
                    dispatch!(cmd)?;
                })*
            }
        }

        delegate!(
            Templates,
            Up,
            New,
            Add,
            Bindle,
            Oci,
            Deploy,
            Build,
            Trigger,
            Login,
            GenerateCompletions
        );

        match self {
            Self::Templates(cmd) => dispatch!(cmd),
            Self::Up(cmd) => dispatch!(cmd),
            Self::New(cmd) => dispatch!(cmd),
            Self::Add(cmd) => dispatch!(cmd),
            Self::Bindle(cmd) => dispatch!(cmd),
            Self::Oci(cmd) => dispatch!(cmd),
            Self::Deploy(cmd) => dispatch!(cmd),
            Self::Build(cmd) => dispatch!(cmd),
            Self::Trigger(cmd) => dispatch!(cmd),
            Self::Login(cmd) => dispatch!(cmd),
            Self::Plugins(cmd) => cmd.dispatch(action).await,
            #[cfg(feature = "generate-completions")]
            Self::GenerateCompletions(cmd) => dispatch!(cmd),
            Self::External(cmd) => external!(cmd)
        }
    }
}

/// Returns build information, similar to: 0.1.0 (2be4034 2022-03-31).
fn build_info() -> String {
    format!(
        "{} ({} {})",
        env!("VERGEN_BUILD_SEMVER"),
        env!("VERGEN_GIT_SHA_SHORT"),
        env!("VERGEN_GIT_COMMIT_DATE")
    )
}
