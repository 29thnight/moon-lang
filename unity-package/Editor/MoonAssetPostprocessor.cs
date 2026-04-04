using System;
using System.IO;
using System.Text.RegularExpressions;
using UnityEditor;
using UnityEngine;

namespace Moon.Editor
{
    /// <summary>
    /// Handles .mn file rename and delete:
    /// - Rename: updates the declared type name and regenerates the C# output
    /// - Delete: removes the corresponding generated .cs and .meta files
    /// </summary>
    public class MoonAssetPostprocessor : AssetPostprocessor
    {
        private static void OnPostprocessAllAssets(
            string[] importedAssets,
            string[] deletedAssets,
            string[] movedAssets,
            string[] movedFromAssetPaths)
        {
            string projectRoot = MoonProjectSettings.GetProjectRoot();
            string outputDir = MoonProjectSettings.GetOutputDir();
            string fullOutputDir = Path.Combine(projectRoot, outputDir);
            Directory.CreateDirectory(fullOutputDir);

            foreach (string deleted in deletedAssets)
            {
                if (IsMoonAssetPath(deleted))
                {
                    DeleteGeneratedScript(fullOutputDir, Path.GetFileNameWithoutExtension(deleted));
                }
            }

            for (int i = 0; i < movedAssets.Length; i++)
            {
                HandleRename(projectRoot, fullOutputDir, movedFromAssetPaths[i], movedAssets[i]);
            }

            if (deletedAssets.Length > 0 || movedAssets.Length > 0)
            {
                EditorApplication.delayCall += () => AssetDatabase.Refresh(ImportAssetOptions.ForceUpdate);
            }
        }

        private static void HandleRename(string projectRoot, string fullOutputDir, string oldPath, string newPath)
        {
            if (!IsMoonAssetPath(oldPath) || !IsMoonAssetPath(newPath))
            {
                return;
            }

            string oldName = Path.GetFileNameWithoutExtension(oldPath);
            string newName = Path.GetFileNameWithoutExtension(newPath);
            if (oldName.Equals(newName, StringComparison.Ordinal))
            {
                return;
            }

            string fullNewPath = Path.Combine(projectRoot, newPath);
            if (!UpdateDeclaredTypeName(fullNewPath, oldName, newName, newPath))
            {
                return;
            }

            DeleteGeneratedScript(fullOutputDir, oldName);

            MoonCompilerBridge.ClearPathCache();
            var compileResult = MoonCompilerBridge.CompileFile(fullNewPath, fullOutputDir);
            if (compileResult.Success)
            {
                Debug.Log($"[Moon] Renamed {oldName} -> {newName}, recompiled to {newName}.cs");
            }
            else
            {
                MoonCompilerBridge.LogDiagnostics(compileResult, newPath);
            }

            string capturedNewName = newName;
            EditorApplication.delayCall += () =>
            {
                AssetDatabase.Refresh(ImportAssetOptions.ForceUpdate);
                MoonIconAssigner.AssignIconToScript(capturedNewName);
            };
        }

        private static bool UpdateDeclaredTypeName(string fullNewPath, string oldName, string newName, string assetPath)
        {
            if (!File.Exists(fullNewPath))
            {
                Debug.LogWarning($"[Moon] Renamed asset is missing on disk: {assetPath}");
                return false;
            }

            try
            {
                string content = File.ReadAllText(fullNewPath);
                string updatedContent = Regex.Replace(
                    content,
                    @"\b(component|asset|class|enum)\s+" + Regex.Escape(oldName) + @"\b",
                    "$1 " + newName);

                if (!string.Equals(content, updatedContent, StringComparison.Ordinal))
                {
                    File.WriteAllText(fullNewPath, updatedContent);
                    Debug.Log($"[Moon] Renamed class {oldName} -> {newName} in {assetPath}");
                }
                else
                {
                    Debug.LogWarning($"[Moon] Renamed {assetPath}, but no declaration matched '{oldName}'.");
                }

                return true;
            }
            catch (Exception ex)
            {
                Debug.LogError($"[Moon] Failed to update renamed Moon script {assetPath}: {ex.Message}");
                return false;
            }
        }

        private static bool DeleteGeneratedScript(string fullOutputDir, string className)
        {
            string csPath = Path.Combine(fullOutputDir, className + ".cs");
            string metaPath = csPath + ".meta";
            bool removedAny = false;

            try
            {
                if (File.Exists(csPath))
                {
                    File.Delete(csPath);
                    Debug.Log($"[Moon] Deleted generated script: {csPath}");
                    removedAny = true;
                }

                if (File.Exists(metaPath))
                {
                    File.Delete(metaPath);
                    removedAny = true;
                }
            }
            catch (Exception ex)
            {
                Debug.LogError($"[Moon] Failed to delete generated script for {className}: {ex.Message}");
            }

            return removedAny;
        }

        private static bool IsMoonAssetPath(string assetPath)
        {
            return assetPath.EndsWith(".mn", StringComparison.OrdinalIgnoreCase);
        }
    }
}
