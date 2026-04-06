using System;
using System.Collections.Generic;
using System.IO;
using System.Text.RegularExpressions;

namespace Prism.Editor
{
    internal static class PrismStackTraceFormatter
    {
        private static readonly Regex DiagnosticLocationRegex = new Regex(
            @"(?m)^(?<path>.*?\.prsm)\((?<line>\d+),(?<col>\d+)\):",
            RegexOptions.Compiled);

        private static readonly Regex PrismFrameRegex = new Regex(
            @"\(at\s+(?<path>.*?\.prsm):(?<line>\d+)\)\s+\[PrSM col\s+(?<col>\d+)\]",
            RegexOptions.Compiled);

        private static readonly Regex DotNetPrismFrameRegex = new Regex(
            @"\sin\s+(?<path>.*?\.prsm):line\s+(?<line>\d+)\s+\[PrSM col\s+(?<col>\d+)\]",
            RegexOptions.Compiled);

        private static readonly Regex UnityFrameRegex = new Regex(
            @"^(?<before>.*\(at\s+)(?<path>.*?\.cs):(?<line>\d+)(?<after>\).*)$",
            RegexOptions.Compiled);

        private static readonly Regex DotNetFrameRegex = new Regex(
            @"^(?<before>.*\sin\s+)(?<path>.*?\.cs):line\s+(?<line>\d+)(?<after>.*)$",
            RegexOptions.Compiled);

        internal static string FormatRemappedRuntimeMessage(string projectRoot, string condition, string stackTrace)
        {
            string remappedStackTrace = RemapStackTrace(projectRoot, stackTrace);
            if (string.IsNullOrWhiteSpace(remappedStackTrace))
            {
                return null;
            }

            string summary = string.IsNullOrWhiteSpace(condition)
                ? "Runtime exception in generated PrSM C#"
                : condition.TrimEnd();

            if (TryExtractFirstPrSMLocation(remappedStackTrace, out string sourcePath, out int sourceLine, out int sourceCol))
            {
                string clickableSummary = $"{sourcePath}({sourceLine},{sourceCol}): error [PrSMRuntime] {summary}";
                return $"{clickableSummary}\n[PrSM] Remapped runtime stack trace from generated PrSM C#\n{remappedStackTrace}";
            }

            return $"[PrSM] Remapped runtime stack trace from generated PrSM C#\n{summary}\n{remappedStackTrace}";
        }

        internal static string RemapStackTrace(string projectRoot, string stackTrace)
        {
            if (string.IsNullOrWhiteSpace(stackTrace))
            {
                return null;
            }

            string normalized = stackTrace.Replace("\r\n", "\n");
            string[] lines = normalized.Split('\n');
            var remappedLines = new List<string>(lines.Length);
            bool changed = false;

            foreach (string line in lines)
            {
                if (TryRemapStackTraceLine(projectRoot, line, out string remappedLine))
                {
                    remappedLines.Add(remappedLine);
                    changed = true;
                }
                else
                {
                    remappedLines.Add(line);
                }
            }

            return changed ? string.Join("\n", remappedLines) : null;
        }

        internal static bool TryRemapStackTraceLine(string projectRoot, string line, out string remappedLine)
        {
            remappedLine = line;

            if (string.IsNullOrWhiteSpace(line))
            {
                return false;
            }

            return TryRemapStackTraceLine(projectRoot, line, UnityFrameRegex, out remappedLine)
                || TryRemapStackTraceLine(projectRoot, line, DotNetFrameRegex, out remappedLine);
        }

        internal static bool TryExtractFirstPrSMLocation(string text, out string sourcePath, out int sourceLine, out int sourceCol)
        {
            sourcePath = null;
            sourceLine = 1;
            sourceCol = 1;

            if (string.IsNullOrWhiteSpace(text))
            {
                return false;
            }

            Match match = DiagnosticLocationRegex.Match(text);
            if (!match.Success)
            {
                match = PrismFrameRegex.Match(text);
                if (!match.Success)
                {
                    match = DotNetPrismFrameRegex.Match(text);
                    if (!match.Success)
                    {
                        return false;
                    }
                }
            }

            sourcePath = match.Groups["path"].Value.Replace('\\', '/');
            sourceLine = ParsePositiveInt(match.Groups["line"].Value);
            sourceCol = ParsePositiveInt(match.Groups["col"].Value);
            return !string.IsNullOrWhiteSpace(sourcePath);
        }

        private static bool TryRemapStackTraceLine(string projectRoot, string line, Regex pattern, out string remappedLine)
        {
            remappedLine = line;
            Match match = pattern.Match(line);
            if (!match.Success)
            {
                return false;
            }

            string generatedFilePath = ResolveGeneratedFilePath(projectRoot, match.Groups["path"].Value);
            if (string.IsNullOrWhiteSpace(generatedFilePath))
            {
                return false;
            }

            if (!int.TryParse(match.Groups["line"].Value, out int generatedLine))
            {
                return false;
            }

            if (!PrismSourceMap.TryResolveSourceLocation(
                projectRoot,
                generatedFilePath,
                generatedLine,
                out string sourcePath,
                out int sourceLine,
                out int sourceCol))
            {
                return false;
            }

            string displayPath = PrismDiagnosticFormatter.GetDisplayPath(projectRoot, sourcePath);
            string before = match.Groups["before"].Value;
            string after = match.Groups["after"].Value;
            remappedLine = $"{before}{displayPath}:{sourceLine}{after} [PrSM col {sourceCol}]";
            return true;
        }

        private static int ParsePositiveInt(string text)
        {
            return int.TryParse(text, out int value) ? Math.Max(1, value) : 1;
        }

        private static string ResolveGeneratedFilePath(string projectRoot, string reportedPath)
        {
            if (string.IsNullOrWhiteSpace(reportedPath))
            {
                return null;
            }

            try
            {
                string fullPath;
                if (Path.IsPathRooted(reportedPath))
                {
                    fullPath = Path.GetFullPath(reportedPath);
                }
                else if (!string.IsNullOrWhiteSpace(projectRoot))
                {
                    fullPath = Path.GetFullPath(Path.Combine(projectRoot, reportedPath));
                }
                else
                {
                    fullPath = Path.GetFullPath(reportedPath);
                }

                if (!fullPath.EndsWith(".cs", StringComparison.OrdinalIgnoreCase))
                {
                    return null;
                }

                string sourceMapPath = PrismSourceMap.GetSourceMapPath(fullPath);
                return File.Exists(sourceMapPath) ? fullPath : null;
            }
            catch
            {
                return null;
            }
        }
    }
}