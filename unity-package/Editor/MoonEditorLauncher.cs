using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using UnityEditor;

namespace Moon.Editor
{
    internal static class MoonEditorLauncher
    {
        internal static void OpenInEditor(string fullPath, int line, int col)
        {
            if (TryLaunchVsCode(fullPath, line, col))
            {
                return;
            }

            EditorUtility.OpenWithDefaultApp(fullPath);
        }

        private static bool TryLaunchVsCode(string fullPath, int line, int col)
        {
            foreach (string candidate in GetVsCodeCandidates())
            {
                try
                {
                    var psi = new ProcessStartInfo
                    {
                        FileName = candidate,
                        Arguments = $"--goto \"{fullPath}\":{Math.Max(1, line)}:{Math.Max(1, col)}",
                        UseShellExecute = true,
                        CreateNoWindow = true,
                    };
                    Process.Start(psi);
                    return true;
                }
                catch
                {
                }
            }

            return false;
        }

        private static IEnumerable<string> GetVsCodeCandidates()
        {
            yield return "code";
            yield return "code.cmd";

            string localAppData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
            if (!string.IsNullOrWhiteSpace(localAppData))
            {
                yield return Path.Combine(localAppData, "Programs", "Microsoft VS Code", "Code.exe");
            }

            string programFiles = Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles);
            if (!string.IsNullOrWhiteSpace(programFiles))
            {
                yield return Path.Combine(programFiles, "Microsoft VS Code", "Code.exe");
            }

            string programFilesX86 = Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86);
            if (!string.IsNullOrWhiteSpace(programFilesX86))
            {
                yield return Path.Combine(programFilesX86, "Microsoft VS Code", "Code.exe");
            }
        }
    }
}