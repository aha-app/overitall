use overitall::{
    config::Config,
    process::ProcessManager,
    procfile::{ProcessSource, ProcfileConfig},
};
use tempfile::TempDir;

#[test]
fn sources_default_and_round_trip() {
    let config: Config = toml::from_str("").unwrap();
    assert!(
        matches!(config.procfile, ProcfileConfig::File(path) if path.to_str() == Some("Procfile"))
    );
    let config: Config = toml::from_str("procfile = 'Procfile.dev'").unwrap();
    assert!(
        matches!(config.procfile, ProcfileConfig::File(path) if path.to_str() == Some("Procfile.dev"))
    );
    let config: Config = toml::from_str(
        "[procfile]\nweb = 'echo hello: world'\n[processes.web]\nrestart = 'always'",
    )
    .unwrap();
    let saved = toml::to_string_pretty(&config).unwrap();
    let restored: Config = toml::from_str(&saved).unwrap();
    assert_eq!(
        restored.procfile.load().unwrap().get_command("web"),
        Some("echo hello: world")
    );
    assert_eq!(restored.processes["web"].restart.as_deref(), Some("always"));
}

#[test]
fn invalid_inline_definitions_fail() {
    for input in [
        "[procfile]",
        "[procfile]\n' ' = 'echo hi'",
        "[procfile]\nweb = ' '",
    ] {
        let config: Config = toml::from_str(input).unwrap();
        assert!(config.procfile.load().is_err(), "{input}");
    }
    for input in [
        "[procfile]\nweb = 3",
        "[procfile]\nweb = 'a'\nweb = 'b'",
        "procfile = 'Procfile'\n[procfile]\nweb = 'a'",
    ] {
        assert!(toml::from_str::<Config>(input).is_err(), "{input}");
    }
}

#[test]
fn inline_reload_and_override_without_procfile() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("config.toml");
    std::fs::write(&path, "[procfile]\nweb = 'echo first'").unwrap();
    let mut config = Config::from_file(path.to_str().unwrap()).unwrap();
    config.config_path = Some(path.clone());
    let source = ProcessSource::resolve(&config, None).unwrap();
    assert_eq!(source.working_dir().unwrap(), temp.path());
    let definitions = source.load().unwrap();
    let mut manager = ProcessManager::new();
    manager.set_process_source(source, temp.path().to_path_buf());
    manager.add_process(
        "web".into(),
        definitions.processes["web"].clone(),
        Some(temp.path().into()),
        None,
        None,
        None,
    );
    std::fs::write(
        &path,
        "[procfile]\nweb = 'echo second'\nworker = 'echo worker'",
    )
    .unwrap();
    let result = manager.reload_procfile(&config).unwrap();
    assert_eq!(result.updated, vec!["web"]);
    assert_eq!(result.added, vec!["worker"]);
    std::fs::write(&path, "[procfile]\nworker = 'echo worker'").unwrap();
    assert_eq!(
        manager.reload_procfile(&config).unwrap().removed,
        vec!["web"]
    );
    std::fs::write(&path, "[procfile]\nworker = ''").unwrap();
    assert!(manager.reload_procfile(&config).is_err());

    let external = temp.path().join("OtherProcfile");
    std::fs::write(&external, "override: echo external").unwrap();
    let source = ProcessSource::resolve(&config, external.to_str()).unwrap();
    assert_eq!(
        source.load().unwrap().get_command("override"),
        Some("echo external")
    );
    assert_eq!(source.working_dir().unwrap(), temp.path());
    assert!(matches!(config.procfile, ProcfileConfig::Inline(_)));
}

#[test]
fn example_inline_config_is_valid() {
    let config: Config = toml::from_str(include_str!("../example/inline.toml")).unwrap();
    let definitions = config.procfile.load().unwrap();
    config
        .validate(&definitions.processes.keys().cloned().collect::<Vec<_>>())
        .unwrap();
    assert_eq!(definitions.get_command("web"), Some("ruby web_server.rb"));
}
