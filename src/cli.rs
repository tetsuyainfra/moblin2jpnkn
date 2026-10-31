use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    /// 設定ファイルのパス
    #[arg(
        short,
        long,
        value_name = "FILE",
        env = "MOBLIN2JPNKN_CONFIG",
        default_value = crate::config::DEFAULT_CONFIG_PATH
    )]
    pub config: String,
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::Parser;

    #[test]
    fn accepts_config_option() {
        let cli = Cli::try_parse_from(["moblin2jpnkn", "--config", "custom.conf"]).unwrap();
        assert_eq!(cli.config, "custom.conf");
    }

    #[test]
    fn rejects_positional_config_path() {
        assert!(Cli::try_parse_from(["moblin2jpnkn", "config_file"]).is_err());
    }
}
