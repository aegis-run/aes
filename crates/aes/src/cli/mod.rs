use clap::{
    Parser, Subcommand,
    builder::styling::{AnsiColor, Effects, Styles},
};

use crate::{context::Context, core::runner::RunResult};

pub mod dump;
pub mod export;

pub(crate) trait Run {
    async fn run(self, ctx: &mut Context<'_>) -> std::io::Result<RunResult>;
}

pub const STYLES: Styles = Styles::styled()
    .header(AnsiColor::BrightGreen.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::BrightGreen.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::BrightCyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default())
    .error(AnsiColor::BrightRed.on_default().effects(Effects::BOLD))
    .valid(AnsiColor::BrightCyan.on_default().effects(Effects::BOLD))
    .invalid(AnsiColor::Yellow.on_default().effects(Effects::BOLD));

#[derive(Debug, Parser)]
#[command(name = "aes", version)]
#[command(styles = STYLES, color = clap::ColorChoice::Always)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Compile and dump schema IR locally
    Dump(dump::DumpArgs),
    /// Validate and publish schema to remote server
    Export(export::ExportArgs),
}

impl Run for Command {
    async fn run(self, ctx: &mut Context<'_>) -> std::io::Result<RunResult> {
        match self {
            Command::Dump(args) => args.run(ctx).await,
            Command::Export(args) => args.run(ctx).await,
        }
    }
}

pub fn print_banner<W: std::io::Write>(w: &mut W) -> std::io::Result<()> {
    let green =
        anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightGreen)));
    let bold_cyan = anstyle::Style::new()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightCyan)))
        .effects(anstyle::Effects::BOLD);
    let cyan = anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Cyan)));
    let white =
        anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightWhite)));
    let reset = anstyle::Reset;

    let version = env!("CARGO_PKG_VERSION");

    writeln!(w)?;
    writeln!(
        w,
        "  {green}    ___       {reset}   {bold_cyan}A E G I S{reset} ({version})"
    )?;
    writeln!(
        w,
        "  {green}   /   | ___  {reset}   {cyan}Open-source Centralized Authorization System{reset}"
    )?;
    writeln!(w, "  {green}  / /| |/ _ \\{reset}")?;
    writeln!(
        w,
        "  {green} / ___ /  __/ {reset}   {white}docs: ................... https://docs.aegis.dev{reset}"
    )?;
    writeln!(
        w,
        "  {green}/_/  |_\\___/  {reset}   {white}github: ..... https://github.com/aegis-run/aegis{reset}"
    )?;
    writeln!(
        w,
        "                   {white}blog: ................... https://aegis.dev/blog{reset}"
    )?;
    writeln!(w)?;
    Ok(())
}
