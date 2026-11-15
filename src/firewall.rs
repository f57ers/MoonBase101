use base64::{engine::general_purpose::STANDARD, Engine};
use bevy::prelude::*;
use std::io;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

#[derive(Component)]
pub struct FirewallStatus;

#[derive(Resource, Default)]
pub struct FirewallAccess {
    process: Option<Child>,
    status: &'static str,
}

impl FirewallAccess {
    pub fn request(&mut self) {
        if self.process.is_some() {
            return;
        }
        match request_elevation() {
            Ok(process) => {
                self.process = Some(process);
                self.status =
                    crate::locale::text("Potwierdź zgodę administratora w oknie Windows.");
            }
            Err(_) => {
                self.status = crate::locale::text("Nie udało się uruchomić konfiguracji zapory.")
            }
        }
    }
}

pub fn update(
    mut access: ResMut<FirewallAccess>,
    mut labels: Query<&mut Text, With<FirewallStatus>>,
) {
    if let Some(process) = &mut access.process {
        access.status = match process.try_wait() {
            Ok(None) => access.status,
            result => {
                access.process = None;
                match result {
                    Ok(Some(code)) if code.success() => {
                        crate::locale::text("Zapora zezwala na dostęp z lokalnej podsieci.")
                    }
                    Ok(Some(code)) if code.code() == Some(2) => {
                        crate::locale::text("Anulowano zgodę administratora.")
                    }
                    _ => crate::locale::text(
                        "Windows nie dodał reguły zapory. Sprawdź uprawnienia administratora.",
                    ),
                }
            }
        };
    }
    for mut label in &mut labels {
        let text = &mut label.sections[0].value;
        if text != access.status {
            access.status.clone_into(text);
        }
    }
}

fn encode_powershell(script: &str) -> String {
    STANDARD.encode(
        script
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
    )
}

fn rule_script(executable: &Path) -> String {
    let program = encode_powershell(&executable.to_string_lossy());
    let lan = crate::net_lan::LAN_PORT;
    let mobile = crate::mobile_host::PORT;
    format!(
        r#"
$ErrorActionPreference = 'Stop'
$exe = [Text.Encoding]::Unicode.GetString([Convert]::FromBase64String('{program}'))
try {{
    $hash = [Security.Cryptography.SHA256]::Create()
    $pathId = [BitConverter]::ToString($hash.ComputeHash([Text.Encoding]::UTF8.GetBytes($exe.ToUpperInvariant()))).Replace('-', '')
    $name = 'MoonBase101-LAN-' + $pathId
    $rule = @{{Name=$name; Program=$exe; Enabled='True'; Direction='Inbound'; Action='Allow'; Protocol='TCP'; LocalPort=@('{lan}', '{mobile}'); RemoteAddress='LocalSubnet'; Profile='Any'}}
    if (Get-NetFirewallRule -Name $name -ErrorAction SilentlyContinue) {{
        Set-NetFirewallRule @rule
    }} else {{
        New-NetFirewallRule @rule -DisplayName 'Moon Base 101 - LAN' | Out-Null
    }}
    exit 0
}} catch {{ exit 1 }}
"#
    )
}

fn request_elevation() -> io::Result<Child> {
    let executable = std::env::current_exe()?;
    let script = encode_powershell(&rule_script(&executable));
    let launcher = encode_powershell(&format!(
        r#"
$ErrorActionPreference = 'Stop'
try {{
    $process = Start-Process -FilePath (Join-Path $PSHOME 'powershell.exe') -Verb RunAs -WindowStyle Hidden -Wait -PassThru -ArgumentList @('-NoProfile', '-NonInteractive', '-EncodedCommand', '{script}')
    exit $process.ExitCode
}} catch {{
    for ($errorCause = $_.Exception; $null -ne $errorCause; $errorCause = $errorCause.InnerException) {{
        if ($errorCause -is [ComponentModel.Win32Exception] -and $errorCause.NativeErrorCode -eq 1223) {{ exit 2 }}
    }}
    exit 1
}}
"#
    ));
    let powershell = PathBuf::from(std::env::var_os("SystemRoot").ok_or(io::ErrorKind::NotFound)?)
        .join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    Command::new(powershell)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            &launcher,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_encodes_the_executable_and_limits_inbound_access() {
        let path = r"E:\Mój folder\O'Brien\Artemis.exe";
        let script = rule_script(Path::new(path));
        assert!(script.contains(&format!("FromBase64String('{}')", encode_powershell(path))));
        assert!(!script.contains(path));
        assert!(script.contains("Direction='Inbound'; Action='Allow'; Protocol='TCP'; LocalPort=@('7878', '7879'); RemoteAddress='LocalSubnet'"));
        assert_eq!(encode_powershell("ął"), "BQFCAQ==");
    }
}
