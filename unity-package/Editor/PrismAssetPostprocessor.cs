using System;
using System.IO;
using System.Text.RegularExpressions;
using UnityEditor;
using UnityEngine;

namespace Prism.Editor
{
    /// <summary>
    /// Handles PrSM source file rename and delete:
    /// - Rename: updates the declared type name and regenerates the C# output
    /// - Delete: removes the corresponding generated .cs and .meta files
    /// </summary>
    public class PrismAssetPostprocessor : AssetPostprocessor
    {
        private static void OnPostprocessAllAssets(
            string[] importedAssets,
            string[] deletedAssets,
            string[] movedAssets,
            string[] movedFromAssetPaths)
        {
            string projectRoot = PrismProjectSettings.GetProjectRoot();
            string outputDir = PrismProjectSettings.GetOutputDir();
            string fullOutputDir = Path.Combine(projectRoot, outputDir);
            Directory.CreateDirectory(fullOutputDir);

            foreach (string deleted in deletedAssets)
            {
                if (IsPrismAssetPath(deleted))
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
            if (!IsPrismAssetPath(oldPath) || !IsPrismAssetPath(newPath))
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

            PrismCompilerBridge.ClearPathCache();
            var compileResult = PrismCompilerBridge.CompileFile(fullNewPath, fullOutputDir);
            if (compileResult.Success)
            {
                Debug.Log($"[PrSM] Renamed {oldName} -> {newName}, recompiled to {newName}.cs");
            }
            else
            {
                PrismCompilerBridge.LogDiagnostics(compileResult, newPath);
            }

            string capturedNewName = newName;
            EditorApplication.delayCall += () =>
            {
                AssetDatabase.Refresh(ImportAssetOptions.ForceUpdate);
                PrismIconAssigner.AssignIconToScript(capturedNewName);
            };
        }

        private static bool UpdateDeclaredTypeName(string fullNewPath, string oldName, string newName, string assetPath)
        {
            if (!File.Exists(fullNewPath))
            {
                Debug.LogWarning($"[PrSM] Renamed asset is missing on disk: {assetPath}");
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
                    Debug.Log($"[PrSM] Renamed class {oldName} -> {newName} in {assetPath}");
                }
                else
                {
                    Debug.LogWarning($"[PrSM] Renamed {assetPath}, but no declaration matched '{oldName}'.");
                }

                return true;
            }
            catch (Exception ex)
            {
                Debug.LogError($"[PrSM] Failed to update renamed PrSM script {assetPath}: {ex.Message}");
                return false;
            }
        }

        private static bool DeleteGeneratedScript(string fullOutputDir, string className)
        {
            string csPath = Path.Combine(fullOutputDir, className + ".cs");
            string metaPath = csPath + ".meta";
            string sourceMapPath = Path.Combine(fullOutputDir, className + ".prsmmap.json");
            string sourceMapMetaPath = sourceMapPath + ".meta";
            bool removedAny = false;

            try
            {
                foreach (string artifactPath in new[] { csPath, metaPath, sourceMapPath, sourceMapMetaPath })
                {
                    if (!File.Exists(artifactPath))
                    {
                        continue;
                    }

                    File.Delete(artifactPath);
                    removedAny = true;
                    if (!artifactPath.EndsWith(".meta", StringComparison.OrdinalIgnoreCase))
                    {
                        Debug.Log($"[PrSM] Deleted generated artifact: {artifactPath}");
                    }
                }
            }
            catch (Exception ex)
            {
                Debug.LogError($"[PrSM] Failed to delete generated script for {className}: {ex.Message}");
            }

            return removedAny;
        }

        private static bool IsPrismAssetPath(string assetPath)
        {
            return PrismProjectConfig.IsPrismSourceAssetPath(assetPath);
        }
    }
}
