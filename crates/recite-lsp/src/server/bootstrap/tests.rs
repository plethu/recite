use std::path::PathBuf;

use recite_config::{LoadedUserConfig, UiConfig, UserConfig};

use super::{
    ConfigError, ServerError, UiCatalog, UiLocale, default_ui_catalog, startup_from_user_config,
};

#[test]
fn valid_user_config_loads_only_the_selected_catalog() -> Result<(), Box<dyn std::error::Error>> {
    let requested = UiLocale::parse("fr-FR")?;
    let loaded = LoadedUserConfig::from_explicit(UserConfig {
        ui: UiConfig {
            locale: requested.clone(),
            ..UiConfig::default()
        },
        ..UserConfig::default()
    });
    let startup = startup_from_user_config(
        Ok(loaded),
        || panic!("valid configuration must not construct the fallback catalog"),
        |locale| {
            assert_eq!(locale, &requested);
            UiCatalog::load(locale).map_err(|error| error.to_string())
        },
    )?;
    assert_eq!(startup.catalog.requested_locale(), &requested.resolve());
    assert!(startup.warning.is_none());
    Ok(())
}

#[test]
fn invalid_user_config_loads_only_the_fallback_and_keeps_the_warning()
-> Result<(), Box<dyn std::error::Error>> {
    let startup = startup_from_user_config(
        Err(ConfigError::Malformed {
            path: PathBuf::from("/fixture/config.toml"),
            message: "invalid TOML".to_owned(),
        }),
        default_ui_catalog,
        |_| panic!("invalid configuration must not load a selected catalog"),
    )?;
    assert_eq!(
        startup.catalog.requested_locale(),
        &UiLocale::default().resolve()
    );
    let warning = startup.warning.ok_or("invalid configuration must warn")?;
    assert!(warning.contains("RECITE_CONFIG005"));
    assert!(warning.contains("/fixture/config.toml"));
    assert!(warning.contains("invalid TOML"));
    Ok(())
}

#[test]
fn selected_catalog_failure_keeps_its_classification_without_loading_fallback()
-> Result<(), Box<dyn std::error::Error>> {
    let requested = UiLocale::parse("fr-FR")?;
    let loaded = LoadedUserConfig::from_explicit(UserConfig {
        ui: UiConfig {
            locale: requested.clone(),
            ..UiConfig::default()
        },
        ..UserConfig::default()
    });
    let result = startup_from_user_config(
        Ok(loaded),
        || panic!("selected catalog failure must not construct the config-error fallback"),
        |locale| {
            assert_eq!(locale, &requested);
            Err("selected catalog could not resolve".to_owned())
        },
    );
    assert!(matches!(
        result,
        Err(ServerError::UiCatalog(message)) if message == "selected catalog could not resolve"
    ));
    Ok(())
}
