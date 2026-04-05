using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using UnityEditor;
using UnityEngine;
using Debug = UnityEngine.Debug;

namespace Prism.Editor
{
    /// <summary>
    /// Wrapper for invoking the prism compiler from Unity Editor.
    /// Resolves the compiler path, executes the CLI, and parses structured diagnostics.
    /// </summary>
    public static class PrismCompilerBridge
    {
        private static string _resolvedPath;

        public static CompileResult CompileFile(string prsmFilePath, string outputDir)
        {
            string args = $"compile \"{prsmFilePath}\" --output \"{outputDir}\" --json";
            return RunCompiler(ResolveCompilerPath(), args);
        }

        public static CompileResult CheckFile(string prsmFilePath)
        {
            string args = $"check \"{prsmFilePath}\" --json";
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

            string overridePath = PrismProjectSettings.GetCompilerPathOverride();
            string configPath = PrismProjectSettings.GetCompilerPath();
            _resolvedPath = PrismCompilerResolver.ResolveCompilerPath(
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
                LogUnityMessage(LogType.Error, $"[PrSM] {result.Stderr.Trim()}");
            }
            else if (!string.IsNullOrWhiteSpace(result?.Stdout))
            {
                LogUnityMessage(LogType.Error, $"[PrSM] {result.Stdout.Trim()}");
            }
        }

        public static void LogDiagnostic(PrismJsonDiagnostic diagnostic, string fallbackPath = null)
        {
            string message = PrismDiagnosticFormatter.FormatDiagnosticMessage(
                PrismProjectSettings.GetProjectRoot(),
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
                result.Stderr = $"Failed to run prism: {e.Message}\nPath: {compilerPath}";
                Debug.LogError($"[PrSM] Failed to run prism.\nPath: {compilerPath}");
            }

            return result;
        }

        private static PrismJsonReport ParseReport(string json)
        {
            try
            {
                var report = JsonUtility.FromJson<PrismJsonReport>(json);
                return report ?? new PrismJsonReport();
            }
            catch
            {
                return new PrismJsonReport();
            }
        }

        private static IEnumerable<string> GetBundledCompilerCandidates()
        {
            var candidates = new List<string>();
            string packageRoot = GetPackageRoot();
            if (!string.IsNullOrWhiteSpace(packageRoot))
            {
                candidates.Add(Path.Combine(packageRoot, "Plugins", "prism~", "prism.exe"));
                candidates.Add(Path.Combine(packageRoot, "Plugins", "prism~", "prism"));

                string repoRoot = Path.GetFullPath(Path.Combine(packageRoot, ".."));
                candidates.Add(Path.Combine(repoRoot, "vscode-prsm", "bin", "prism.exe"));
                candidates.Add(Path.Combine(repoRoot, "vscode-prsm", "bin", "prism"));
            }

            string home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
            string extensionsRoot = Path.Combine(home, ".vscode", "extensions");
            if (Directory.Exists(extensionsRoot))
            {
                foreach (string extensionDir in Directory.GetDirectories(extensionsRoot, "prsm-lang.prsm-lang-*"))
                {
                    candidates.Add(Path.Combine(extensionDir, "bin", "prism.exe"));
                    candidates.Add(Path.Combine(extensionDir, "bin", "prism"));
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
                Path.Combine(repoRoot, "target", "debug", "prism.exe"),
                Path.Combine(repoRoot, "target", "debug", "prism"),
                Path.Combine(repoRoot, "target", "release", "prism.exe"),
                Path.Combine(repoRoot, "target", "release", "prism"),
            };
        }

        private static string GetPackageRoot()
        {
            var packageInfo = UnityEditor.PackageManager.PackageInfo.FindForAssembly(typeof(PrismCompilerBridge).Assembly);
            return packageInfo?.resolvedPath;
        }
    }

    [Serializable]
    public class PrismJsonDiagnostic
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
    public class PrismJsonReport
    {
        public string project = "";
        public int files;
        public int compiled;
        public int errors;
        public int warnings;
        public string output_dir = "";
        public PrismJsonDiagnostic[] diagnostics = Array.Empty<PrismJsonDiagnostic>();
    }

    public class CompileResult
    {
        public bool Success;
        public int ExitCode;
        public string Stdout = "";
        public string Stderr = "";
        public string JsonOutput = "";
        public PrismJsonReport Report = new PrismJsonReport();

        public PrismJsonDiagnostic[] Diagnostics => Report?.diagnostics ?? Array.Empty<PrismJsonDiagnostic>();
    }
}
