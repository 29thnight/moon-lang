using System;
using System.IO;

namespace Moon.Editor
{
    internal static class MoonProjectConfig
    {
        internal static string ResolveProjectPath(string projectRoot, string candidatePath)
        {
            if (string.IsNullOrWhiteSpace(candidatePath) || candidatePath == "moonc")
            {
                return candidatePath;
            }

            return Path.IsPathRooted(candidatePath)
                ? candidatePath
                : Path.GetFullPath(Path.Combine(projectRoot, candidatePath));
        }

        internal static string ParseTomlValue(string content, string key, string section = null)
        {
            if (string.IsNullOrWhiteSpace(content))
            {
                return null;
            }

            string[] lines = content.Split(new[] { "\r\n", "\n" }, StringSplitOptions.None);
            bool inSection = section == null;

            foreach (string rawLine in lines)
            {
                string trimmed = rawLine.Trim();

                if (trimmed.StartsWith("[") && trimmed.EndsWith("]"))
                {
                    string sectionName = trimmed.Substring(1, trimmed.Length - 2).Trim();
                    inSection = section == null || sectionName == section;
                    continue;
                }

                if (!inSection || string.IsNullOrEmpty(trimmed) || trimmed.StartsWith("#"))
                {
                    continue;
                }

                int eq = trimmed.IndexOf('=');
                if (eq <= 0)
                {
                    continue;
                }

                string parsedKey = trimmed.Substring(0, eq).Trim();
                if (parsedKey != key)
                {
                    continue;
                }

                string value = trimmed.Substring(eq + 1).Trim();
                if (value.StartsWith("\"") && value.EndsWith("\""))
                {
                    value = value.Substring(1, value.Length - 2);
                }
                return value;
            }

            return null;
        }
    }
}