//! AndroidManifest.xml Parser
//!
//! Parses Android manifest files into structured data.

use std::path::Path;
use quick_xml::Reader;
use quick_xml::events::{Event, BytesStart};
use tracing::{debug, warn};

use crate::manifest::{AndroidManifest, ManifestApplication, ManifestMetadata};
use crate::permissions::Permission;
use crate::components::{Activity, Service, Receiver, Provider};
use crate::intent_filters::{IntentFilter, IntentAction, IntentCategory, IntentData};

/// Parser errors
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("XML parsing error: {0}")]
    XmlError(#[from] quick_xml::Error),
    #[error("Invalid manifest structure: {0}")]
    InvalidStructure(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

/// Android namespace prefix
const ANDROID_NS: &str = "http://schemas.android.com/apk/res/android";

/// Manifest parser
pub struct ManifestParser;

impl ManifestParser {
    /// Parse a manifest file from path
    pub async fn parse_file(path: impl AsRef<Path>) -> Result<AndroidManifest, ParseError> {
        let content = tokio::fs::read_to_string(path.as_ref()).await?;
        Self::parse_string(&content)
    }

    /// Parse manifest from string
    pub fn parse_string(xml: &str) -> Result<AndroidManifest, ParseError> {
        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

        let mut manifest = AndroidManifest::default();
        let mut buf = Vec::new();
        let mut current_activity: Option<Activity> = None;
        let mut current_service: Option<Service> = None;
        let mut current_receiver: Option<Receiver> = None;
        let mut current_provider: Option<Provider> = None;
        let mut current_intent_filter: Option<IntentFilter> = None;
        let mut in_application = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let is_empty = matches!(reader.read_event_into(&mut Vec::new()), Ok(Event::Empty(_)));
                    
                    match e.name().as_ref() {
                        b"manifest" => {
                            Self::parse_manifest_attrs(&mut manifest, e)?;
                        }
                        b"uses-sdk" => {
                            Self::parse_uses_sdk(&mut manifest, e)?;
                        }
                        b"uses-permission" => {
                            if let Some(perm) = Self::parse_permission(e)? {
                                manifest.permissions.push(perm);
                            }
                        }
                        b"uses-permission-sdk-23" => {
                            if let Some(perm) = Self::parse_permission(e)? {
                                manifest.permissions_sdk23.push(perm);
                            }
                        }
                        b"uses-feature" => {
                            if let Some((name, required)) = Self::parse_uses_feature(e)? {
                                manifest.features.push((name, required));
                            }
                        }
                        b"application" => {
                            in_application = true;
                            manifest.application = Some(Self::parse_application(e)?);
                        }
                        b"activity" if in_application => {
                            current_activity = Some(Self::parse_activity(e)?);
                        }
                        b"activity-alias" if in_application => {
                            // Handle activity alias similar to activity
                            current_activity = Some(Self::parse_activity(e)?);
                        }
                        b"service" if in_application => {
                            current_service = Some(Self::parse_service(e)?);
                        }
                        b"receiver" if in_application => {
                            current_receiver = Some(Self::parse_receiver(e)?);
                        }
                        b"provider" if in_application => {
                            current_provider = Some(Self::parse_provider(e)?);
                        }
                        b"intent-filter" => {
                            current_intent_filter = Some(IntentFilter::default());
                        }
                        b"action" => {
                            if let Some(ref mut filter) = current_intent_filter {
                                if let Some(action) = Self::parse_action(e)? {
                                    filter.actions.push(action);
                                }
                            }
                        }
                        b"category" => {
                            if let Some(ref mut filter) = current_intent_filter {
                                if let Some(category) = Self::parse_category(e)? {
                                    filter.categories.push(category);
                                }
                            }
                        }
                        b"data" => {
                            if let Some(ref mut filter) = current_intent_filter {
                                if let Some(data) = Self::parse_data(e)? {
                                    filter.data.push(data);
                                }
                            }
                        }
                        b"meta-data" => {
                            if let Some(meta) = Self::parse_metadata(e)? {
                                if let Some(ref mut act) = current_activity {
                                    act.metadata.push(meta);
                                } else if let Some(ref mut svc) = current_service {
                                    svc.metadata.push(meta);
                                } else if let Some(ref mut app) = manifest.application {
                                    app.metadata.push(meta);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    match e.name().as_ref() {
                        b"application" => {
                            in_application = false;
                        }
                        b"activity" | b"activity-alias" => {
                            if let Some(mut activity) = current_activity.take() {
                                if let Some(ref mut app) = manifest.application {
                                    app.activities.push(activity);
                                }
                            }
                        }
                        b"service" => {
                            if let Some(service) = current_service.take() {
                                if let Some(ref mut app) = manifest.application {
                                    app.services.push(service);
                                }
                            }
                        }
                        b"receiver" => {
                            if let Some(receiver) = current_receiver.take() {
                                if let Some(ref mut app) = manifest.application {
                                    app.receivers.push(receiver);
                                }
                            }
                        }
                        b"provider" => {
                            if let Some(provider) = current_provider.take() {
                                if let Some(ref mut app) = manifest.application {
                                    app.providers.push(provider);
                                }
                            }
                        }
                        b"intent-filter" => {
                            if let Some(filter) = current_intent_filter.take() {
                                if let Some(ref mut act) = current_activity {
                                    act.intent_filters.push(filter);
                                } else if let Some(ref mut svc) = current_service {
                                    svc.intent_filters.push(filter);
                                } else if let Some(ref mut rcv) = current_receiver {
                                    rcv.intent_filters.push(filter);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(ParseError::XmlError(e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(manifest)
    }

    /// Get an android: namespaced attribute
    fn get_android_attr(e: &BytesStart, name: &str) -> Option<String> {
        let android_name = format!("android:{}", name);
        for attr in e.attributes().filter_map(|a| a.ok()) {
            let key = std::str::from_utf8(attr.key.as_ref()).ok()?;
            if key == android_name {
                return std::str::from_utf8(&attr.value).ok().map(|s| s.to_string());
            }
        }
        None
    }

    /// Get a non-namespaced attribute
    fn get_attr(e: &BytesStart, name: &str) -> Option<String> {
        for attr in e.attributes().filter_map(|a| a.ok()) {
            let key = std::str::from_utf8(attr.key.as_ref()).ok()?;
            if key == name {
                return std::str::from_utf8(&attr.value).ok().map(|s| s.to_string());
            }
        }
        None
    }

    fn parse_manifest_attrs(manifest: &mut AndroidManifest, e: &BytesStart) -> Result<(), ParseError> {
        manifest.package = Self::get_attr(e, "package").unwrap_or_default();
        manifest.version_code = Self::get_android_attr(e, "versionCode")
            .and_then(|s| s.parse().ok());
        manifest.version_name = Self::get_android_attr(e, "versionName");
        manifest.install_location = Self::get_android_attr(e, "installLocation");
        Ok(())
    }

    fn parse_uses_sdk(manifest: &mut AndroidManifest, e: &BytesStart) -> Result<(), ParseError> {
        manifest.min_sdk = Self::get_android_attr(e, "minSdkVersion")
            .and_then(|s| s.parse().ok());
        manifest.target_sdk = Self::get_android_attr(e, "targetSdkVersion")
            .and_then(|s| s.parse().ok());
        manifest.max_sdk = Self::get_android_attr(e, "maxSdkVersion")
            .and_then(|s| s.parse().ok());
        Ok(())
    }

    fn parse_permission(e: &BytesStart) -> Result<Option<Permission>, ParseError> {
        let name = Self::get_android_attr(e, "name");
        if let Some(name) = name {
            let max_sdk = Self::get_android_attr(e, "maxSdkVersion")
                .and_then(|s| s.parse().ok());
            Ok(Some(Permission {
                name,
                max_sdk_version: max_sdk,
            }))
        } else {
            Ok(None)
        }
    }

    fn parse_uses_feature(e: &BytesStart) -> Result<Option<(String, bool)>, ParseError> {
        let name = Self::get_android_attr(e, "name");
        if let Some(name) = name {
            let required = Self::get_android_attr(e, "required")
                .map(|s| s != "false")
                .unwrap_or(true);
            Ok(Some((name, required)))
        } else {
            Ok(None)
        }
    }

    fn parse_application(e: &BytesStart) -> Result<ManifestApplication, ParseError> {
        Ok(ManifestApplication {
            name: Self::get_android_attr(e, "name"),
            label: Self::get_android_attr(e, "label"),
            icon: Self::get_android_attr(e, "icon"),
            round_icon: Self::get_android_attr(e, "roundIcon"),
            theme: Self::get_android_attr(e, "theme"),
            allow_backup: Self::get_android_attr(e, "allowBackup")
                .map(|s| s == "true"),
            supports_rtl: Self::get_android_attr(e, "supportsRtl")
                .map(|s| s == "true"),
            use_cleartext_traffic: Self::get_android_attr(e, "usesCleartextTraffic")
                .map(|s| s == "true"),
            ..Default::default()
        })
    }

    fn parse_activity(e: &BytesStart) -> Result<Activity, ParseError> {
        Ok(Activity {
            name: Self::get_android_attr(e, "name").unwrap_or_default(),
            label: Self::get_android_attr(e, "label"),
            exported: Self::get_android_attr(e, "exported")
                .map(|s| s == "true"),
            theme: Self::get_android_attr(e, "theme"),
            screen_orientation: Self::get_android_attr(e, "screenOrientation"),
            launch_mode: Self::get_android_attr(e, "launchMode"),
            config_changes: Self::get_android_attr(e, "configChanges"),
            ..Default::default()
        })
    }

    fn parse_service(e: &BytesStart) -> Result<Service, ParseError> {
        Ok(Service {
            name: Self::get_android_attr(e, "name").unwrap_or_default(),
            exported: Self::get_android_attr(e, "exported")
                .map(|s| s == "true"),
            enabled: Self::get_android_attr(e, "enabled")
                .map(|s| s == "true")
                .unwrap_or(true),
            foreground_service_type: Self::get_android_attr(e, "foregroundServiceType"),
            ..Default::default()
        })
    }

    fn parse_receiver(e: &BytesStart) -> Result<Receiver, ParseError> {
        Ok(Receiver {
            name: Self::get_android_attr(e, "name").unwrap_or_default(),
            exported: Self::get_android_attr(e, "exported")
                .map(|s| s == "true"),
            enabled: Self::get_android_attr(e, "enabled")
                .map(|s| s == "true")
                .unwrap_or(true),
            ..Default::default()
        })
    }

    fn parse_provider(e: &BytesStart) -> Result<Provider, ParseError> {
        Ok(Provider {
            name: Self::get_android_attr(e, "name").unwrap_or_default(),
            authorities: Self::get_android_attr(e, "authorities").unwrap_or_default(),
            exported: Self::get_android_attr(e, "exported")
                .map(|s| s == "true"),
            grant_uri_permissions: Self::get_android_attr(e, "grantUriPermissions")
                .map(|s| s == "true"),
            ..Default::default()
        })
    }

    fn parse_action(e: &BytesStart) -> Result<Option<IntentAction>, ParseError> {
        Ok(Self::get_android_attr(e, "name").map(|name| IntentAction { name }))
    }

    fn parse_category(e: &BytesStart) -> Result<Option<IntentCategory>, ParseError> {
        Ok(Self::get_android_attr(e, "name").map(|name| IntentCategory { name }))
    }

    fn parse_data(e: &BytesStart) -> Result<Option<IntentData>, ParseError> {
        let data = IntentData {
            scheme: Self::get_android_attr(e, "scheme"),
            host: Self::get_android_attr(e, "host"),
            port: Self::get_android_attr(e, "port"),
            path: Self::get_android_attr(e, "path"),
            path_prefix: Self::get_android_attr(e, "pathPrefix"),
            path_pattern: Self::get_android_attr(e, "pathPattern"),
            mime_type: Self::get_android_attr(e, "mimeType"),
        };
        
        // Only return if at least one field is set
        if data.scheme.is_some() || data.host.is_some() || data.mime_type.is_some() {
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    fn parse_metadata(e: &BytesStart) -> Result<Option<ManifestMetadata>, ParseError> {
        let name = Self::get_android_attr(e, "name");
        let value = Self::get_android_attr(e, "value");
        let resource = Self::get_android_attr(e, "resource");
        
        if let Some(name) = name {
            Ok(Some(ManifestMetadata { name, value, resource }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_MANIFEST: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.example.app"
    android:versionCode="1"
    android:versionName="1.0">
    
    <uses-sdk android:minSdkVersion="24" android:targetSdkVersion="34"/>
    <uses-permission android:name="android.permission.INTERNET"/>
    <uses-feature android:name="android.hardware.camera" android:required="false"/>
    
    <application
        android:label="@string/app_name"
        android:icon="@mipmap/ic_launcher"
        android:theme="@style/Theme.App">
        
        <activity
            android:name=".MainActivity"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN"/>
                <category android:name="android.intent.category.LAUNCHER"/>
            </intent-filter>
        </activity>
    </application>
</manifest>"#;

    #[test]
    fn test_parse_manifest() {
        let manifest = ManifestParser::parse_string(SAMPLE_MANIFEST).unwrap();
        
        assert_eq!(manifest.package, "com.example.app");
        assert_eq!(manifest.version_code, Some(1));
        assert_eq!(manifest.min_sdk, Some(24));
        assert_eq!(manifest.target_sdk, Some(34));
        assert_eq!(manifest.permissions.len(), 1);
        assert_eq!(manifest.permissions[0].name, "android.permission.INTERNET");
    }
}
