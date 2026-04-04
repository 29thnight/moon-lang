using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using UnityEditor;
using UnityEngine;
using Debug = UnityEngine.Debug;

namespace Moon.Editor
{
    /// <summary>
    /// Wrapper for invoking the moonc compiler from Unity Editor.
    /// Resolves the compiler path, executes the CLI, and parses structured diagnostics.
    /// </summary>
    public static class MoonCompilerBridge
    {
        private static string _resolvedPath;

        public static CompileResult CompileFile(string moonFilePath, string outputDir)
        {
            string args = $"compile \"{moonFilePath}\" --output \"{outputDir}\" --json";
            return RunCompiler(ResolveCompilerPath(), args);
        }

        public static CompileResult CheckFile(string moonFilePath)
        {
            string args = $"check \"{moonFilePath}\" --json";
            return RunCompiler(ResolveCompilerPath(), args);
        }

        public static CompileResult BuildProject()
        {
            return RunCompiler(ResolveCompilerPath(), "build --json");
        }

        public static string ResolveCompilerPath()
        {
            if (!string.IsNullOrWhiteSpace(_resolvedPath))
            {
                return _resolvedPath;
            }

            string overridePath = MoonProjectSettings.GetCompilerPathOverride();
            string configPath = MoonProjectSettings.GetCompilerPath();
            _resolvedPath = MoonCompilerResolver.ResolveCompilerPath(
                overridePath,
                configPath,
                GetBundledCompilerCandidates(),
                GetDevelopmentCompilerCandidates(),
                File.Exists);
            return _resolvedPath;
        }

        public static void ClearPathCache()
        {
            _resolvedPath = null;
        }

        public static void LogDiagnostics(CompileResult result, string fallbackPath = null)
        {
            if (result?.Diagnostics != null && result.Diagnostics.Length > 0)
            {
                foreach (var diagnostic in result.Diagnostics)
                {
                    LogDiagnostic(diagnostic, fallbackPath);
                }
                return;
            }

            if (!string.IsNullOrWhiteSpace(result?.Stderr))
            {
                LogUnityMessage(LogType.Error, $"[Moon] {result.Stderr.Trim()}");
            }
            else if (!string.IsNullOrWhiteSpace(result?.Stdout))
            {
                LogUnityMessage(LogType.Error, $"[Moon] {result.Stdout.Trim()}");
            }
        }

        public static void LogDiagnostic(MoonJsonDiagnostic diagnostic, string fallbackPath = null)
        {
            string message = MoonDiagnosticFormatter.FormatDiagnosticMessage(
                MoonProjectSettings.GetProjectRoot(),
                diagnostic,
                fallbackPath);
            LogUnityMessage(diagnostic?.severity == "warning" ? LogType.Warning : LogType.Error, message);
        }

        private static void LogUnityMessage(LogType logType, string message)
        {
            Debug.LogFormat(logType, LogOption.NoStacktrace, null, "{0}", message);
        }

        private static CompileResult RunCompiler(string compilerPath, string arguments)
        {
            var result = new CompileResult();

            try
            {
                var psi = new ProcessStartInfo
                {
                    FileName = compilerPath,
                    Arguments = arguments,
                    UseShellExecute = false,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    CreateNoWindow = true,
                    WorkingDirectory = Path.GetDirectoryName(Application.dataPath)
                };

                using (var process = Process.Start(psi))
                {
                    string stdout = process.StandardOutput.ReadToEnd();
                    string stderr = process.StandardError.ReadToEnd();
                    process.WaitForExit(30000);

                    result.ExitCode = process.ExitCode;
                    result.Stdout = stdout;
                    result.Stderr = stderr;
                    result.Success = process.ExitCode == 0;

                    string trimmed = stdout.Trim();
                    if (!string.IsNullOrEmpty(trimmed) && trimmed.StartsWith("{"))
                    {
                        result.JsonOutput = trimmed;
                        result.Report = ParseReport(trimmed);
                    }
                }
            }
            catch (Exception e)
            {
                result.Success = false;
                result.Stderr = $"Failed to run moonc: {e.Message}\nPath: {compilerPath}";
                Debug.LogError($"[Moon] Failed to run moonc.\nPath: {compilerPath}");
            }

            return result;
        }

        private static MoonJsonReport ParseReport(string json)
        {
            try
            {
                var report = JsonUtility.FromJson<MoonJsonReport>(json);
                return report ?? new MoonJsonReport();
            }
            catch
            {
                return new MoonJsonReport();
            }
        }

        private static IEnumerable<string> GetBundledCompilerCandidates()
        {
            var candidates = new List<string>();
            string packageRoot = GetPackageRoot();
            if (!string.IsNullOrWhiteSpace(packageRoot))
            {
                candidates.Add(Path.Combine(packageRoot, "Plugins", "moonc~", "moonc.exe"));
                candidates.Add(Path.Combine(packageRoot, "Plugins", "moonc~", "moonc"));

                string repoRoot = Path.GetFullPath(Path.Combine(packageRoot, ".."));
                candidates.Add(Path.Combine(repoRoot, "vscode-moon", "bin", "moonc.exe"));
                candidates.Add(Path.Combine(repoRoot, "vscode-moon", "bin", "moonc"));
            }

            string home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
            string extensionsRoot = Path.Combine(home, ".vscode", "extensions");
            if (Directory.Exists(extensionsRoot))
            {
                foreach (string extensionDir in Directory.GetDirectories(extensionsRoot, "moon-lang.moon-lang-*"))
                {
                    candidates.Add(Path.Combine(extensionDir, "bin", "moonc.exe"));
                    candidates.Add(Path.Combine(extensionDir, "bin", "moonc"));
                }
            }

            return candidates;
        }

        private static IEnumerable<string> GetDevelopmentCompilerCandidates()
        {
            string packageRoot = GetPackageRoot();
            if (string.IsNullOrWhiteSpace(packageRoot))
            {
                return Array.Empty<string>();
            }

            string repoRoot = Path.GetFullPath(Path.Combine(packageRoot, ".."));
            return new[]
            {
                Path.Combine(repoRoot, "target", "debug", "moonc.exe"),
                Path.Combine(repoRoot, "target", "debug", "moonc"),
                Path.Combine(repoRoot, "target", "release", "moonc.exe"),
                Path.Combine(repoRoot, "target", "release", "moonc"),
            };
        }

        private static string GetPackageRoot()
        {
            var packageInfo = UnityEditor.PackageManager.PackageInfo.FindForAssembly(typeof(MoonCompilerBridge).Assembly);
            return packageInfo?.resolvedPath;
        }
    }

    [Serializable]
    public class MoonJsonDiagnostic
    {
        public string code = "";
        public string severity = "";
        public string message = "";
        public string file = "";
        public int line;
        public int col;
        public int end_line;
        public int end_col;
    }

    [Serializable]
    public class MoonJsonReport
    {
        public string project = "";
        public int files;
        public int compiled;
        public int errors;
        public int warnings;
        public string output_dir = "";
        public MoonJsonDiagnostic[] diagnostics = Array.Empty<MoonJsonDiagnostic>();
    }

    public class CompileResult
    {
        public bool Success;
        public int ExitCode;
        public string Stdout = "";
        public string Stderr = "";
        public string JsonOutput = "";
        public MoonJsonReport Report = new MoonJsonReport();

        public MoonJsonDiagnostic[] Diagnostics => Report?.diagnostics ?? Array.Empty<MoonJsonDiagnostic>();
    }
}
