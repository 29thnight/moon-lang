using System;
using System.IO;

namespace Moon.Editor
{
    internal static class MoonDiagnosticFormatter
    {
        internal static string FormatDiagnosticMessage(string projectRoot, MoonJsonDiagnostic diagnostic, string fallbackPath = null)
        {
            string displayPath = GetDisplayPath(projectRoot, diagnostic?.file, fallbackPath);
            int line = Math.Max(1, diagnostic?.line ?? 1);
            int col = Math.Max(1, diagnostic?.col ?? 1);
            string severity = string.IsNullOrWhiteSpace(diagnostic?.severity) ? "error" : diagnostic.severity;
            string code = string.IsNullOrWhiteSpace(diagnostic?.code) ? "E000" : diagnostic.code;
            string message = diagnostic?.message ?? string.Empty;

            return $"{displayPath}({line},{col}): {severity} [{code}] {message}";
        }

        internal static string GetDisplayPath(string projectRoot, string reportedPath, string fallbackPath = null)
        {
            string pathToFormat = string.IsNullOrWhiteSpace(reportedPath) ? fallbackPath : reportedPath;
            if (string.IsNullOrWhiteSpace(pathToFormat))
            {
                return "Unknown.mn";
            }

            try
            {
                if (string.IsNullOrWhiteSpace(projectRoot))
                {
                    return pathToFormat.Replace('\\', '/');
                }

                string fullPath = Path.IsPathRooted(pathToFormat)
                    ? pathToFormat
                    : Path.Combine(projectRoot, pathToFormat);
                string relativePath = Path.GetRelativePath(projectRoot, fullPath);
                return relativePath.Replace('\\', '/');
            }
            catch
            {
                return pathToFormat.Replace('\\', '/');
            }
        }
    }
}