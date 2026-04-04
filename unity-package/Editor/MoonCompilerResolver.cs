using System;
using System.Collections.Generic;
using System.Linq;

namespace Moon.Editor
{
    internal static class MoonCompilerResolver
    {
        internal static string ResolveCompilerPath(
            string overridePath,
            string configuredPath,
            IEnumerable<string> bundledCandidates,
            IEnumerable<string> developmentCandidates,
            Func<string, bool> exists)
        {
            foreach (string candidate in EnumerateCandidates(overridePath, configuredPath, bundledCandidates, developmentCandidates))
            {
                if (!string.IsNullOrWhiteSpace(candidate) && exists(candidate))
                {
                    return candidate;
                }
            }

            return "moonc";
        }

        private static IEnumerable<string> EnumerateCandidates(
            string overridePath,
            string configuredPath,
            IEnumerable<string> bundledCandidates,
            IEnumerable<string> developmentCandidates)
        {
            if (!string.IsNullOrWhiteSpace(overridePath))
            {
                yield return overridePath;
            }

            if (!string.IsNullOrWhiteSpace(configuredPath) && configuredPath != "moonc")
            {
                yield return configuredPath;
            }

            foreach (string candidate in developmentCandidates ?? Enumerable.Empty<string>())
            {
                yield return candidate;
            }

            foreach (string candidate in bundledCandidates ?? Enumerable.Empty<string>())
            {
                yield return candidate;
            }
        }
    }
}