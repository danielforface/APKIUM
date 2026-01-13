//! Manifest Writer
//!
//! Writes AndroidManifest.xml from structured data.

use std::path::Path;
use quick_xml::{Writer, events::{Event, BytesStart, BytesEnd, BytesText, BytesDecl}};
use std::io::Cursor;
use tracing::info;

use crate::manifest::{AndroidManifest, ManifestApplication, ManifestMetadata};
use crate::permissions::Permission;
use crate::components::{Activity, Service, Receiver, Provider};
use crate::intent_filters::{IntentFilter, IntentAction, IntentCategory, IntentData};

/// Writer errors
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error("XML write error: {0}")]
    XmlError(#[from] quick_xml::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

/// Manifest writer
pub struct ManifestWriter {
    indent: usize,
}

impl ManifestWriter {
    /// Create a new writer with default settings
    pub fn new() -> Self {
        Self { indent: 4 }
    }

    /// Set indentation
    pub fn with_indent(mut self, spaces: usize) -> Self {
        self.indent = spaces;
        self
    }

    /// Write manifest to string
    pub fn write_to_string(&self, manifest: &AndroidManifest) -> Result<String, WriteError> {
        let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', self.indent);

        // XML declaration
        writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;
        writer.write_event(Event::Text(BytesText::from_escaped("\n")))?;

        // Manifest element
        let mut manifest_elem = BytesStart::new("manifest");
        manifest_elem.push_attribute(("xmlns:android", "http://schemas.android.com/apk/res/android"));
        manifest_elem.push_attribute(("package", manifest.package.as_str()));
        
        if let Some(code) = manifest.version_code {
            manifest_elem.push_attribute(("android:versionCode", code.to_string().as_str()));
        }
        if let Some(ref name) = manifest.version_name {
            manifest_elem.push_attribute(("android:versionName", name.as_str()));
        }
        if let Some(ref loc) = manifest.install_location {
            manifest_elem.push_attribute(("android:installLocation", loc.as_str()));
        }

        writer.write_event(Event::Start(manifest_elem))?;

        // uses-sdk
        if manifest.min_sdk.is_some() || manifest.target_sdk.is_some() {
            let mut uses_sdk = BytesStart::new("uses-sdk");
            if let Some(min) = manifest.min_sdk {
                uses_sdk.push_attribute(("android:minSdkVersion", min.to_string().as_str()));
            }
            if let Some(target) = manifest.target_sdk {
                uses_sdk.push_attribute(("android:targetSdkVersion", target.to_string().as_str()));
            }
            if let Some(max) = manifest.max_sdk {
                uses_sdk.push_attribute(("android:maxSdkVersion", max.to_string().as_str()));
            }
            writer.write_event(Event::Empty(uses_sdk))?;
        }

        // Permissions
        for perm in &manifest.permissions {
            self.write_permission(&mut writer, perm, false)?;
        }
        for perm in &manifest.permissions_sdk23 {
            self.write_permission(&mut writer, perm, true)?;
        }

        // Features
        for (name, required) in &manifest.features {
            let mut feature = BytesStart::new("uses-feature");
            feature.push_attribute(("android:name", name.as_str()));
            if !*required {
                feature.push_attribute(("android:required", "false"));
            }
            writer.write_event(Event::Empty(feature))?;
        }

        // Queries
        if !manifest.queries.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("queries")))?;
            for query in &manifest.queries {
                match query {
                    crate::manifest::Query::Package(pkg) => {
                        let mut elem = BytesStart::new("package");
                        elem.push_attribute(("android:name", pkg.as_str()));
                        writer.write_event(Event::Empty(elem))?;
                    }
                    crate::manifest::Query::Intent(filter) => {
                        writer.write_event(Event::Start(BytesStart::new("intent")))?;
                        self.write_intent_filter_contents(&mut writer, filter)?;
                        writer.write_event(Event::End(BytesEnd::new("intent")))?;
                    }
                    crate::manifest::Query::Provider(authority) => {
                        let mut elem = BytesStart::new("provider");
                        elem.push_attribute(("android:authorities", authority.as_str()));
                        writer.write_event(Event::Empty(elem))?;
                    }
                }
            }
            writer.write_event(Event::End(BytesEnd::new("queries")))?;
        }

        // Application
        if let Some(ref app) = manifest.application {
            self.write_application(&mut writer, app)?;
        }

        writer.write_event(Event::End(BytesEnd::new("manifest")))?;

        let result = writer.into_inner().into_inner();
        Ok(String::from_utf8(result)?)
    }

    /// Write manifest to file
    pub async fn write_to_file(&self, manifest: &AndroidManifest, path: impl AsRef<Path>) -> Result<(), WriteError> {
        let content = self.write_to_string(manifest)?;
        tokio::fs::write(path.as_ref(), content).await?;
        info!("Wrote manifest to {:?}", path.as_ref());
        Ok(())
    }

    fn write_permission<W: std::io::Write>(&self, writer: &mut Writer<W>, perm: &Permission, sdk23: bool) -> Result<(), WriteError> {
        let tag = if sdk23 { "uses-permission-sdk-23" } else { "uses-permission" };
        let mut elem = BytesStart::new(tag);
        elem.push_attribute(("android:name", perm.name.as_str()));
        if let Some(max) = perm.max_sdk_version {
            elem.push_attribute(("android:maxSdkVersion", max.to_string().as_str()));
        }
        writer.write_event(Event::Empty(elem))?;
        Ok(())
    }

    fn write_application<W: std::io::Write>(&self, writer: &mut Writer<W>, app: &ManifestApplication) -> Result<(), WriteError> {
        let mut elem = BytesStart::new("application");
        
        if let Some(ref name) = app.name {
            elem.push_attribute(("android:name", name.as_str()));
        }
        if let Some(ref label) = app.label {
            elem.push_attribute(("android:label", label.as_str()));
        }
        if let Some(ref icon) = app.icon {
            elem.push_attribute(("android:icon", icon.as_str()));
        }
        if let Some(ref round_icon) = app.round_icon {
            elem.push_attribute(("android:roundIcon", round_icon.as_str()));
        }
        if let Some(ref theme) = app.theme {
            elem.push_attribute(("android:theme", theme.as_str()));
        }
        if let Some(backup) = app.allow_backup {
            elem.push_attribute(("android:allowBackup", if backup { "true" } else { "false" }));
        }
        if let Some(rtl) = app.supports_rtl {
            elem.push_attribute(("android:supportsRtl", if rtl { "true" } else { "false" }));
        }
        if let Some(cleartext) = app.use_cleartext_traffic {
            elem.push_attribute(("android:usesCleartextTraffic", if cleartext { "true" } else { "false" }));
        }
        if let Some(ref network_config) = app.network_security_config {
            elem.push_attribute(("android:networkSecurityConfig", network_config.as_str()));
        }
        if let Some(hw_accel) = app.hardware_accelerated {
            elem.push_attribute(("android:hardwareAccelerated", if hw_accel { "true" } else { "false" }));
        }
        if let Some(large_heap) = app.large_heap {
            elem.push_attribute(("android:largeHeap", if large_heap { "true" } else { "false" }));
        }

        writer.write_event(Event::Start(elem))?;

        // Metadata
        for meta in &app.metadata {
            self.write_metadata(writer, meta)?;
        }

        // Activities
        for activity in &app.activities {
            self.write_activity(writer, activity)?;
        }

        // Services
        for service in &app.services {
            self.write_service(writer, service)?;
        }

        // Receivers
        for receiver in &app.receivers {
            self.write_receiver(writer, receiver)?;
        }

        // Providers
        for provider in &app.providers {
            self.write_provider(writer, provider)?;
        }

        writer.write_event(Event::End(BytesEnd::new("application")))?;
        Ok(())
    }

    fn write_activity<W: std::io::Write>(&self, writer: &mut Writer<W>, activity: &Activity) -> Result<(), WriteError> {
        let mut elem = BytesStart::new("activity");
        elem.push_attribute(("android:name", activity.name.as_str()));
        
        if let Some(ref label) = activity.label {
            elem.push_attribute(("android:label", label.as_str()));
        }
        if let Some(exported) = activity.exported {
            elem.push_attribute(("android:exported", if exported { "true" } else { "false" }));
        }
        if let Some(ref theme) = activity.theme {
            elem.push_attribute(("android:theme", theme.as_str()));
        }
        if let Some(ref orientation) = activity.screen_orientation {
            elem.push_attribute(("android:screenOrientation", orientation.as_str()));
        }
        if let Some(ref launch_mode) = activity.launch_mode {
            elem.push_attribute(("android:launchMode", launch_mode.as_str()));
        }
        if let Some(ref config_changes) = activity.config_changes {
            elem.push_attribute(("android:configChanges", config_changes.as_str()));
        }
        if let Some(ref window_soft_input) = activity.window_soft_input_mode {
            elem.push_attribute(("android:windowSoftInputMode", window_soft_input.as_str()));
        }

        if activity.intent_filters.is_empty() && activity.metadata.is_empty() {
            writer.write_event(Event::Empty(elem))?;
        } else {
            writer.write_event(Event::Start(elem))?;
            
            for meta in &activity.metadata {
                self.write_metadata(writer, meta)?;
            }
            
            for filter in &activity.intent_filters {
                self.write_intent_filter(writer, filter)?;
            }
            
            writer.write_event(Event::End(BytesEnd::new("activity")))?;
        }
        
        Ok(())
    }

    fn write_service<W: std::io::Write>(&self, writer: &mut Writer<W>, service: &Service) -> Result<(), WriteError> {
        let mut elem = BytesStart::new("service");
        elem.push_attribute(("android:name", service.name.as_str()));
        
        if let Some(exported) = service.exported {
            elem.push_attribute(("android:exported", if exported { "true" } else { "false" }));
        }
        if !service.enabled {
            elem.push_attribute(("android:enabled", "false"));
        }
        if let Some(ref permission) = service.permission {
            elem.push_attribute(("android:permission", permission.as_str()));
        }
        if let Some(ref fg_type) = service.foreground_service_type {
            elem.push_attribute(("android:foregroundServiceType", fg_type.as_str()));
        }

        if service.intent_filters.is_empty() && service.metadata.is_empty() {
            writer.write_event(Event::Empty(elem))?;
        } else {
            writer.write_event(Event::Start(elem))?;
            
            for meta in &service.metadata {
                self.write_metadata(writer, meta)?;
            }
            
            for filter in &service.intent_filters {
                self.write_intent_filter(writer, filter)?;
            }
            
            writer.write_event(Event::End(BytesEnd::new("service")))?;
        }
        
        Ok(())
    }

    fn write_receiver<W: std::io::Write>(&self, writer: &mut Writer<W>, receiver: &Receiver) -> Result<(), WriteError> {
        let mut elem = BytesStart::new("receiver");
        elem.push_attribute(("android:name", receiver.name.as_str()));
        
        if let Some(exported) = receiver.exported {
            elem.push_attribute(("android:exported", if exported { "true" } else { "false" }));
        }
        if !receiver.enabled {
            elem.push_attribute(("android:enabled", "false"));
        }
        if let Some(ref permission) = receiver.permission {
            elem.push_attribute(("android:permission", permission.as_str()));
        }

        if receiver.intent_filters.is_empty() && receiver.metadata.is_empty() {
            writer.write_event(Event::Empty(elem))?;
        } else {
            writer.write_event(Event::Start(elem))?;
            
            for meta in &receiver.metadata {
                self.write_metadata(writer, meta)?;
            }
            
            for filter in &receiver.intent_filters {
                self.write_intent_filter(writer, filter)?;
            }
            
            writer.write_event(Event::End(BytesEnd::new("receiver")))?;
        }
        
        Ok(())
    }

    fn write_provider<W: std::io::Write>(&self, writer: &mut Writer<W>, provider: &Provider) -> Result<(), WriteError> {
        let mut elem = BytesStart::new("provider");
        elem.push_attribute(("android:name", provider.name.as_str()));
        elem.push_attribute(("android:authorities", provider.authorities.as_str()));
        
        if let Some(exported) = provider.exported {
            elem.push_attribute(("android:exported", if exported { "true" } else { "false" }));
        }
        if !provider.enabled {
            elem.push_attribute(("android:enabled", "false"));
        }
        if let Some(grant) = provider.grant_uri_permissions {
            elem.push_attribute(("android:grantUriPermissions", if grant { "true" } else { "false" }));
        }
        if let Some(ref read_perm) = provider.read_permission {
            elem.push_attribute(("android:readPermission", read_perm.as_str()));
        }
        if let Some(ref write_perm) = provider.write_permission {
            elem.push_attribute(("android:writePermission", write_perm.as_str()));
        }

        if provider.metadata.is_empty() && provider.path_permissions.is_empty() {
            writer.write_event(Event::Empty(elem))?;
        } else {
            writer.write_event(Event::Start(elem))?;
            
            for meta in &provider.metadata {
                self.write_metadata(writer, meta)?;
            }
            
            writer.write_event(Event::End(BytesEnd::new("provider")))?;
        }
        
        Ok(())
    }

    fn write_intent_filter<W: std::io::Write>(&self, writer: &mut Writer<W>, filter: &IntentFilter) -> Result<(), WriteError> {
        let mut elem = BytesStart::new("intent-filter");
        
        if let Some(auto_verify) = filter.auto_verify {
            if auto_verify {
                elem.push_attribute(("android:autoVerify", "true"));
            }
        }
        
        writer.write_event(Event::Start(elem))?;
        self.write_intent_filter_contents(writer, filter)?;
        writer.write_event(Event::End(BytesEnd::new("intent-filter")))?;
        
        Ok(())
    }

    fn write_intent_filter_contents<W: std::io::Write>(&self, writer: &mut Writer<W>, filter: &IntentFilter) -> Result<(), WriteError> {
        for action in &filter.actions {
            let mut elem = BytesStart::new("action");
            elem.push_attribute(("android:name", action.name.as_str()));
            writer.write_event(Event::Empty(elem))?;
        }
        
        for category in &filter.categories {
            let mut elem = BytesStart::new("category");
            elem.push_attribute(("android:name", category.name.as_str()));
            writer.write_event(Event::Empty(elem))?;
        }
        
        for data in &filter.data {
            let mut elem = BytesStart::new("data");
            
            if let Some(ref scheme) = data.scheme {
                elem.push_attribute(("android:scheme", scheme.as_str()));
            }
            if let Some(ref host) = data.host {
                elem.push_attribute(("android:host", host.as_str()));
            }
            if let Some(ref port) = data.port {
                elem.push_attribute(("android:port", port.as_str()));
            }
            if let Some(ref path) = data.path {
                elem.push_attribute(("android:path", path.as_str()));
            }
            if let Some(ref path_prefix) = data.path_prefix {
                elem.push_attribute(("android:pathPrefix", path_prefix.as_str()));
            }
            if let Some(ref path_pattern) = data.path_pattern {
                elem.push_attribute(("android:pathPattern", path_pattern.as_str()));
            }
            if let Some(ref mime_type) = data.mime_type {
                elem.push_attribute(("android:mimeType", mime_type.as_str()));
            }
            
            writer.write_event(Event::Empty(elem))?;
        }
        
        Ok(())
    }

    fn write_metadata<W: std::io::Write>(&self, writer: &mut Writer<W>, meta: &ManifestMetadata) -> Result<(), WriteError> {
        let mut elem = BytesStart::new("meta-data");
        elem.push_attribute(("android:name", meta.name.as_str()));
        
        if let Some(ref value) = meta.value {
            elem.push_attribute(("android:value", value.as_str()));
        }
        if let Some(ref resource) = meta.resource {
            elem.push_attribute(("android:resource", resource.as_str()));
        }
        
        writer.write_event(Event::Empty(elem))?;
        Ok(())
    }
}

impl Default for ManifestWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_basic_manifest() {
        let manifest = AndroidManifest::new("com.example.test");
        let writer = ManifestWriter::new();
        let xml = writer.write_to_string(&manifest).unwrap();
        
        assert!(xml.contains("com.example.test"));
        assert!(xml.contains("uses-sdk"));
        assert!(xml.contains("application"));
    }

    #[test]
    fn test_roundtrip() {
        let mut manifest = AndroidManifest::new("com.example.app");
        manifest.add_permission("android.permission.INTERNET");
        
        if let Some(ref mut app) = manifest.application {
            app.activities.push(Activity::launcher(".MainActivity"));
        }
        
        let writer = ManifestWriter::new();
        let xml = writer.write_to_string(&manifest).unwrap();
        
        // Parse it back
        let parsed = crate::parser::ManifestParser::parse_string(&xml).unwrap();
        
        assert_eq!(parsed.package, manifest.package);
        assert_eq!(parsed.permissions.len(), 1);
    }
}
