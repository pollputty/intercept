#[cfg(test)]
mod tests {
    use std::io::{Read, Seek, SeekFrom};

    use intercept::{config, SpawnOptions, Tracer};

    fn test_config() -> config::Config {
        config::Config {
            log: config::LogConfig {
                level: config::LogLevel::INFO,
            },
            record: config::RecordConfig {
                path: "/dev/null".into(),
                files: false,
                random: false,
                time: false,
            },
            redirect: config::RedirectConfig {
                files: vec![],
                random: false,
                time: None,
            },
        }
    }

    fn run_command(conf: &config::Config, command: &str, args: &[&str]) -> std::io::Result<String> {
        let mut tmp = tempfile::tempfile()?;
        let clone = tmp.try_clone()?;

        let opts = SpawnOptions {
            stdout: Some(clone.into()),
            stderr: None,
        };
        let tracer = Tracer::spawn(command, args, Some(opts))?;
        tracer.run(conf)?;
        let mut output = String::new();
        tmp.seek(SeekFrom::Start(0))?;
        tmp.read_to_string(&mut output)?;
        Ok(output)
    }

    #[test]
    fn echo() {
        let conf = test_config();
        let result = run_command(&conf, "echo", &["foo"]);
        assert!(result.is_ok());
        assert_eq!("foo", result.unwrap().trim());
    }

    #[test]
    fn cat() {
        let conf = test_config();
        let result = run_command(&conf, "cat", &["/etc/passwd"]);
        assert!(result.is_ok());
        assert!(!result.unwrap().trim().is_empty());
    }

    #[test]
    fn intercepted_cat() {
        let mut conf = test_config();
        conf.redirect.files.push(config::Redirect {
            from: "/etc/passwd".to_string(),
            to: "/dev/null".to_string(),
        });
        let result = run_command(&conf, "cat", &["/etc/passwd"]);
        assert!(result.is_ok());
        assert!(result.unwrap().trim().is_empty());
    }

    #[test]
    fn subcommand() {
        let conf = test_config();
        let result = run_command(&conf, "bash", &["-c", "echo hello"]);
        assert!(result.is_ok());
        assert_eq!("hello", result.unwrap().trim());
    }

    #[test]
    fn python3() {
        let conf = test_config();
        let result = run_command(&conf, "python3", &["-c", "print('hello')"]);
        assert!(result.is_ok());
        assert_eq!("hello", result.unwrap().trim());
    }
}
