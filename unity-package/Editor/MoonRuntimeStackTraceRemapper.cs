using System;
using System.Collections.Generic;
using System.IO;
using UnityEditor;
using UnityEngine;

namespace Moon.Editor
{
    [InitializeOnLoad]
    internal static class MoonRuntimeStackTraceRemapper
    {
        private static bool _isEmittingRemap;

        // Maps absolute .mn source path → (line, col) for the most recently emitted remapped log.
        // Populated when the remapped log is emitted; consumed by MoonScriptProxy.OnOpenMoonAsset
        // to work around m_ActiveText being empty at click time.
        private static readonly Dictionary<string, (int line, int col)> s_LocationCache =
            new Dictionary<string, (int line, int col)>(StringComparer.OrdinalIgnoreCase);

        static MoonRuntimeStackTraceRemapper()
        {
            Application.logMessageReceived -= HandleLogMessage;
            Application.logMessageReceived += HandleLogMessage;
        }

        private static void HandleLogMessage(string condition, string stackTrace, LogType type)
        {
            if (_isEmittingRemap)
            {
                return;
            }

            if (type != LogType.Error && type != LogType.Assert && type != LogType.Exception)
            {
                return;
            }

            string remappedMessage = MoonStackTraceFormatter.FormatRemappedRuntimeMessage(
                MoonProjectSettings.GetProjectRoot(),
                condition,
                stackTrace);
            if (string.IsNullOrWhiteSpace(remappedMessage))
            {
                return;
            }

            string projectRoot = MoonProjectSettings.GetProjectRoot();

            // Cache the source location so OnOpenMoonAsset can navigate correctly even when
            // m_ActiveText is empty at click time (console loses focus on click).
            if (MoonStackTraceFormatter.TryExtractFirstMoonLocation(remappedMessage, out string cachedSourcePath, out int cachedLine, out int cachedCol))
            {
                string fullSourcePath = ResolveFullSourcePath(projectRoot, cachedSourcePath);
                if (!string.IsNullOrWhiteSpace(fullSourcePath))
                {
                    s_LocationCache[fullSourcePath] = (cachedLine, cachedCol);
                }
            }

            try
            {
                _isEmittingRemap = true;
                Debug.LogFormat(type, LogOption.NoStacktrace, LoadSourceContext(projectRoot, remappedMessage), "{0}", remappedMessage);
            }
            finally
            {
                _isEmittingRemap = false;
            }
        }

        /// <summary>
        /// Returns the cached (line, col) for a .mn source file if it was populated from a remapped
        /// runtime log. The entry is removed after being consumed so it won't stick forever.
        /// </summary>
        internal static bool TryConsumeCachedLocation(string fullSourcePath, out int line, out int col)
        {
            line = 1;
            col = 1;
            if (string.IsNullOrWhiteSpace(fullSourcePath))
            {
                return false;
            }

            if (s_LocationCache.TryGetValue(fullSourcePath, out var loc))
            {
                s_LocationCache.Remove(fullSourcePath);
                line = loc.line;
                col = loc.col;
                return true;
            }

            return false;
        }

        private static string ResolveFullSourcePath(string projectRoot, string sourcePath)
        {
            if (string.IsNullOrWhiteSpace(sourcePath))
            {
                return null;
            }

            if (Path.IsPathRooted(sourcePath))
            {
                return Path.GetFullPath(sourcePath);
            }

            if (!string.IsNullOrWhiteSpace(projectRoot))
            {
                return Path.GetFullPath(Path.Combine(projectRoot, sourcePath));
            }

            return Path.GetFullPath(sourcePath);
        }

        internal static bool TryResolveRemappedAssetPath(string projectRoot, string remappedMessage, out string assetPath)
        {
            assetPath = null;

            if (!MoonStackTraceFormatter.TryExtractFirstMoonLocation(remappedMessage, out string sourcePath, out _, out _))
            {
                return false;
            }

            assetPath = ToAssetPath(projectRoot, sourcePath);
            return !string.IsNullOrWhiteSpace(assetPath);
        }

        private static UnityEngine.Object LoadSourceContext(string projectRoot, string remappedMessage)
        {
            if (!TryResolveRemappedAssetPath(projectRoot, remappedMessage, out string assetPath))
            {
                return null;
            }

            return AssetDatabase.LoadAssetAtPath<UnityEngine.Object>(assetPath);
        }

        private static string ToAssetPath(string projectRoot, string sourcePath)
        {
            if (string.IsNullOrWhiteSpace(sourcePath))
            {
                return null;
            }

            string normalized = sourcePath.Replace('\\', '/');
            if (!Path.IsPathRooted(sourcePath))
            {
                return IsProjectAssetPath(normalized) ? normalized : null;
            }

            if (string.IsNullOrWhiteSpace(projectRoot))
            {
                return null;
            }

            try
            {
                string fullProjectRoot = Path.GetFullPath(projectRoot)
                    .TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
                string fullSourcePath = Path.GetFullPath(sourcePath)
                    .TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);

                if (!fullSourcePath.StartsWith(fullProjectRoot + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase)
                    && !fullSourcePath.StartsWith(fullProjectRoot + Path.AltDirectorySeparatorChar, StringComparison.OrdinalIgnoreCase))
                {
                    return null;
                }

                string relativePath = Path.GetRelativePath(fullProjectRoot, fullSourcePath).Replace('\\', '/');
                return IsProjectAssetPath(relativePath) ? relativePath : null;
            }
            catch
            {
                return null;
            }
        }

        private static bool IsProjectAssetPath(string path)
        {
            return path.StartsWith("Assets/", StringComparison.OrdinalIgnoreCase)
                || path.StartsWith("Packages/", StringComparison.OrdinalIgnoreCase)
                || string.Equals(path, "Assets", StringComparison.OrdinalIgnoreCase)
                || string.Equals(path, "Packages", StringComparison.OrdinalIgnoreCase);
        }
    }
}