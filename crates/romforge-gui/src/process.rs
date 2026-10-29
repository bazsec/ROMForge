use std::{ffi::OsStr, process::Command};

/// Keep helper output piped into ROMForge without creating a foreground console.
pub fn background(program: impl AsRef<OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn console_helper_has_no_window_and_retains_both_output_streams() {
        let script = r#"Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public class ConsoleProbe { [DllImport("kernel32.dll")] public static extern IntPtr GetConsoleWindow(); }'; if ([ConsoleProbe]::GetConsoleWindow() -ne [IntPtr]::Zero) { exit 9 }; [Console]::Out.Write('captured output'); [Console]::Error.Write('captured error')"#;
        let output = background("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                script,
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"captured output");
        assert_eq!(output.stderr, b"captured error");
    }
}
