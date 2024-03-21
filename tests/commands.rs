#[cfg(test)]
mod tests {
    use intercept::config;
    use intercept::tracer::{run, spawn};

    fn test_config() -> config::Config {
        config::Config {
            log: config::LogConfig {
                level: config::LogLevel::INFO,
            },
            redirect: config::RedirectConfig {
                files: vec![],
                random: false,
            },
        }
    }

    #[test]
    fn echo() {
        let conf = test_config();
        assert!(spawn("echo", ["foo"]).is_ok());
        assert!(run(&conf).is_ok());
    }

    #[test]
    fn cat() {
        let conf = test_config();
        assert!(spawn("cat", ["/etc/passwd"]).is_ok());
        assert!(run(&conf).is_ok());
    }

    #[test]
    fn intercepted_cat() {
        let mut conf = test_config();
        conf.redirect.files.push(config::Redirect {
            from: "/etc/passwd".to_string(),
            to: "/dev/null".to_string(),
        });
        assert!(spawn("cat", ["/etc/passwd"]).is_ok());
        assert!(run(&conf).is_ok());
    }

    #[test]
    fn subcommand() {
        let conf = test_config();
        assert!(spawn("bash", ["-c", "echo a"]).is_ok());
        assert!(run(&conf).is_ok());
    }

    #[test]
    fn python3() {
        let conf = test_config();
        assert!(spawn("python3", ["-c", "print('hello')"]).is_ok());
        assert!(run(&conf).is_ok());
    }
}
