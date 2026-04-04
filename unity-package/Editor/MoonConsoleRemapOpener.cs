using System;
using System.IO;
using System.Reflection;
using System.Text.RegularExpressions;
using UnityEditor;

namespace Moon.Editor
{
    internal static class MoonConsoleRemapOpener
    {
        private static readonly Regex DiagnosticLocationRegex = new Regex(
            @"(?m)^(?<path>.*?\.mn)\((?<line>\d+),(?<col>\d+)\):",
            RegexOptions.Compiled);

        private static readonly Regex MoonFrameRegex = new Regex(
            @"\(at\s+(?<path>.*?\.mn):(?<line>\d+)\)\s+\[Moon col\s+(?<col>\d+)\]",
            RegexOptions.Compiled);

        private static readonly Regex DotNetMoonFrameRegex = new Regex(
            @"\sin\s+(?<path>.*?\.mn):line\s+(?<line>\d+)\s+\[Moon col\s+(?<col>\d+)\]",
            RegexOptions.Compiled);

        internal static bool TryOpenSelectedRemappedFrame(string projectRoot)
        {
            string activeText = GetSelectedConsoleText();
            if (!TryParseFirstMoonLocation(activeText, out string sourcePath, out int sourceLine, out int sourceCol))
            {
                return false;
            }

            string fullPath = ResolveSourcePath(projectRoot, sourcePath);
            if (string.IsNullOrWhiteSpace(fullPath) || !File.Exists(fullPath))
            {
                return false;
            }

            MoonEditorLauncher.OpenInEditor(fullPath, sourceLine, sourceCol);
            return true;
        }

        internal static bool TryGetSelectedLocationForAsset(string projectRoot, string assetPath, out int sourceLine, out int sourceCol)
        {
            sourceLine = 1;
            sourceCol = 1;

            string activeText = GetSelectedConsoleText();
            if (!TryParseFirstMoonLocation(activeText, out string sourcePath, out int parsedLine, out int parsedCol))
            {
                return false;
            }

            string fullPath = ResolveSourcePath(projectRoot, sourcePath);
            if (!PathsEqual(fullPath, assetPath))
            {
                return false;
            }

            sourceLine = parsedLine;
            sourceCol = parsedCol;
            return true;
        }

        internal static bool TryParseFirstMoonLocation(string text, out string sourcePath, out int sourceLine, out int sourceCol)
        {
            if (TryParseMatch(DiagnosticLocationRegex.Match(text ?? string.Empty), out sourcePath, out sourceLine, out sourceCol))
            {
                return true;
            }

            if (TryParseMatch(DotNetMoonFrameRegex.Match(text ?? string.Empty), out sourcePath, out sourceLine, out sourceCol))
            {
                return true;
            }

            return TryParseFirstMoonFrame(text, out sourcePath, out sourceLine, out sourceCol);
        }

        internal static bool TryParseFirstMoonFrame(string text, out string sourcePath, out int sourceLine, out int sourceCol)
        {
            sourcePath = null;
            sourceLine = 1;
            sourceCol = 1;

            if (string.IsNullOrWhiteSpace(text))
            {
                return false;
            }

            return TryParseMatch(MoonFrameRegex.Match(text), out sourcePath, out sourceLine, out sourceCol);
        }

        private static string GetSelectedConsoleText()
        {
            try
            {
                Assembly editorAssembly = typeof(EditorWindow).Assembly;
                Type consoleWindowType = editorAssembly.GetType("UnityEditor.ConsoleWindow");
                if (consoleWindowType == null)
                {
                    return null;
                }

                FieldInfo consoleField = consoleWindowType.GetField("ms_ConsoleWindow", BindingFlags.Static | BindingFlags.NonPublic);
                EditorWindow consoleWindow = consoleField?.GetValue(null) as EditorWindow;
                if (consoleWindow == null)
                {
                    return null;
                }

                FieldInfo activeTextField = consoleWindowType.GetField("m_ActiveText", BindingFlags.Instance | BindingFlags.NonPublic);
                return activeTextField?.GetValue(consoleWindow) as string;
            }
            catch
            {
                return null;
            }
        }

        private static string ResolveSourcePath(string projectRoot, string sourcePath)
        {
            if (string.IsNullOrWhiteSpace(sourcePath))
            {
                return null;
            }

            if (Path.IsPathRooted(sourcePath))
            {
                return sourcePath;
            }

            if (string.IsNullOrWhiteSpace(projectRoot))
            {
                return Path.GetFullPath(sourcePath);
            }

            return Path.GetFullPath(Path.Combine(projectRoot, sourcePath));
        }

        private static int ParsePositiveInt(string text)
        {
            return int.TryParse(text, out int value) ? Math.Max(1, value) : 1;
        }

        private static bool TryParseMatch(Match match, out string sourcePath, out int sourceLine, out int sourceCol)
        {
            sourcePath = null;
            sourceLine = 1;
            sourceCol = 1;

            if (match == null || !match.Success)
            {
                return false;
            }

            sourcePath = match.Groups["path"].Value.Replace('/', Path.DirectorySeparatorChar);
            sourceLine = ParsePositiveInt(match.Groups["line"].Value);
            sourceCol = ParsePositiveInt(match.Groups["col"].Value);
            return !string.IsNullOrWhiteSpace(sourcePath);
        }

        private static bool PathsEqual(string left, string right)
        {
            if (string.IsNullOrWhiteSpace(left) || string.IsNullOrWhiteSpace(right))
            {
                return false;
            }

            try
            {
                string normalizedLeft = Path.GetFullPath(left)
                    .TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
                string normalizedRight = Path.GetFullPath(right)
                    .TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
                return string.Equals(normalizedLeft, normalizedRight, StringComparison.OrdinalIgnoreCase);
            }
            catch
            {
                return false;
            }
        }
    }
}