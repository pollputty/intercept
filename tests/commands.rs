#[cfg(test)]
mod tests {
    use intercept::{config, Tracer};

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
        let tracer = Tracer::spawn("echo", ["foo"]).unwrap();
        assert!(tracer.run(&conf).is_ok());
    }

    #[test]
    fn cat() {
        let conf = test_config();
        let tracer = Tracer::spawn("cat", ["/etc/passwd"]).unwrap();
        assert!(tracer.run(&conf).is_ok());
    }

    #[test]
    fn intercepted_cat() {
        let mut conf = test_config();
        conf.redirect.files.push(config::Redirect {
            from: "/etc/passwd".to_string(),
            to: "/dev/null".to_string(),
        });
        let tracer = Tracer::spawn("cat", ["/etc/passwd"]).unwrap();
        assert!(tracer.run(&conf).is_ok());
    }

    #[test]
    fn subcommand() {
        let conf = test_config();
        let tracer = Tracer::spawn("bash", ["-c", "echo a"]).unwrap();
        assert!(tracer.run(&conf).is_ok());
    }

    #[test]
    fn python3() {
        let conf = test_config();
        let tracer = Tracer::spawn("python3", ["-c", "print('hello')"]).unwrap();
        assert!(tracer.run(&conf).is_ok());
    }
}
