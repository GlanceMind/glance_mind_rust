use glance_mind_protocol::glance_mind::ViduVideoConfig;

#[test]
fn vidu_config_deserializes_mode_and_quality() {
    let json = r#"{"style":"general","movement_amplitude":"auto","bgm":true,"model_version":"viduq2","mode":"oneclick","quality":"fast"}"#;
    let cfg: ViduVideoConfig = serde_json::from_str(json).expect("deserialize ViduVideoConfig");
    assert_eq!(cfg.mode, "oneclick");
    assert_eq!(cfg.quality, "fast");
    let back = serde_json::to_string(&cfg).expect("serialize");
    assert!(back.contains("\"mode\":\"oneclick\""), "got: {back}");
    assert!(back.contains("\"quality\":\"fast\""), "got: {back}");
}

#[test]
fn vidu_config_defaults_when_fields_absent() {
    let json = r#"{"style":"general","movement_amplitude":"auto","bgm":true,"model_version":"viduq2"}"#;
    let cfg: ViduVideoConfig = serde_json::from_str(json).expect("deserialize legacy");
    assert_eq!(cfg.mode, "");
    assert_eq!(cfg.quality, "");
}
