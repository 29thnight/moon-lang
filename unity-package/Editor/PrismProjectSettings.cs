using System.IO;
using System;
using UnityEngine;
using UnityEditor;

namespace Prism.Editor
{
    /// <summary>
    /// Reads/creates .prsmproject settings.
    /// Generated C# goes to Packages/com.prsm.generated/ — hidden from Assets/.
    /// </summary>
    public static class PrismProjectSettings
    {
        private const string GeneratedPackageName = "com.prsm.generated";
        private const string GeneratedPackageDir = "Packages/com.prsm.generated";
        private const string LegacyGeneratedPackageDir = "Packages/com.moon.generated";
        private const string CompilerPathOverrideKey = "PrSM.CompilerPathOverride";

        private static string _cachedCompilerPath;
        private static string _cachedOutputDir;

        public static string GetProjectRoot()
        {
            return Path.GetDirectoryName(Application.dataPath);
        }

        public static string GetProjectFilePath()
        {
            return Path.Combine(GetProjectRoot(), PrismProjectConfig.ProjectFileName);
        }

        public static string GetLegacyProjectFilePath()
        {
            return Path.Combine(GetProjectRoot(), PrismProjectConfig.LegacyProjectFileName);
        }

        public static string GetActiveProjectFilePath()
        {
            return PrismProjectConfig.FindProjectFilePath(GetProjectRoot());
        }

        public static bool HasProjectFile()
        {
            return File.Exists(GetProjectFilePath());
        }

        public static bool HasLegacyProjectFile()
        {
            return File.Exists(GetLegacyProjectFilePath());
        }

        public static void EnsureProjectFile()
        {
            if (HasProjectFile()) return;

            if (MigrateLegacyProjectIfNeeded())
            {
                EnsureGeneratedPackage();
                return;
            }

            string projectName = Path.GetFileName(GetProjectRoot());
            string content = $@"[project]
name = ""{projectName}""
prsm_version = ""0.1.0""

[compiler]
prism_path = ""prism""
output_dir = ""Packages/com.prsm.generated/Runtime""

[source]
include = [""Assets/**/*.prsm""]
exclude = []

[features]
auto_compile_on_save = true
generate_meta_files = true
pascal_case_methods = true
";
            File.WriteAllText(GetProjectFilePath(), content);
            Debug.Log($"[PrSM] Created {PrismProjectConfig.ProjectFileName} at {GetProjectRoot()}");

            EnsureGeneratedPackage();
        }

        public static bool MigrateLegacyProjectIfNeeded()
        {
            if (HasProjectFile() || !HasLegacyProjectFile())
            {
                return false;
            }

            string legacyPath = GetLegacyProjectFilePath();
            string migratedPath = GetProjectFilePath();
            string legacyContent = File.ReadAllText(legacyPath);
            string normalizedContent = PrismProjectConfig.NormalizeProjectConfigContent(legacyContent);
            File.WriteAllText(migratedPath, normalizedContent);
            ClearCache();
            Debug.Log($"[PrSM] Migrated legacy {PrismProjectConfig.LegacyProjectFileName} to {PrismProjectConfig.ProjectFileName}.");
            return true;
        }

        /// <summary>
        /// Ensures the generated code UPM package exists.
        /// </summary>
        public static void EnsureGeneratedPackage()
        {
            string root = GetProjectRoot();
            string pkgDir = Path.Combine(root, GeneratedPackageDir, "Runtime");
            string pkgJson = Path.Combine(root, GeneratedPackageDir, "package.json");
            string asmdef = Path.Combine(root, GeneratedPackageDir, "Runtime", "Prism.Generated.asmdef");

            if (!Directory.Exists(pkgDir))
                Directory.CreateDirectory(pkgDir);

            if (!File.Exists(pkgJson))
            {
                File.WriteAllText(pkgJson, @"{
  ""name"": ""com.prsm.generated"",
  ""version"": ""0.0.1"",
  ""displayName"": ""PrSM Generated Scripts"",
  ""description"": ""Auto-generated C# scripts from PrSM (.prsm) source files. Do not edit."",
  ""unity"": ""2022.3""
}
");
                Debug.Log("[PrSM] Created generated package: " + GeneratedPackageDir);
            }

            if (!File.Exists(asmdef))
            {
                File.WriteAllText(asmdef, @"{
    ""name"": ""Prism.Generated"",
    ""rootNamespace"": """",
    ""references"": [],
    ""includePlatforms"": [],
    ""excludePlatforms"": [],
    ""allowUnsafeCode"": false,
    ""overrideReferences"": false,
    ""precompiledReferences"": [],
    ""autoReferenced"": true,
    ""defineConstraints"": [],
    ""versionDefines"": [],
    ""noEngineReferences"": false
}
");
            }
        }

        public static bool MoveLegacyGeneratedPackageToBackup()
        {
            string projectRoot = GetProjectRoot();
            string legacyPackageDir = Path.Combine(projectRoot, LegacyGeneratedPackageDir);
            if (!Directory.Exists(legacyPackageDir))
            {
                return false;
            }

            string backupRoot = Path.Combine(projectRoot, "Library", "PrismMigrationBackup");
            Directory.CreateDirectory(backupRoot);

            string backupDir = Path.Combine(
                backupRoot,
                $"com.moon.generated_{DateTime.UtcNow:yyyyMMddHHmmss}");
            Directory.Move(legacyPackageDir, backupDir);
            Debug.Log($"[PrSM] Moved legacy generated package to {backupDir}");
            return true;
        }

        public static string GetCompilerPath()
        {
            if (_cachedCompilerPath != null) return _cachedCompilerPath;

            MigrateLegacyProjectIfNeeded();

            string path = PrismProjectConfig.NormalizeCompilerPath(ReadTomlValue(new[] { "prism_path", "moonc_path" }, "compiler"));
            if (string.IsNullOrEmpty(path) || path == PrismProjectConfig.DefaultCompilerPath)
            {
                _cachedCompilerPath = PrismProjectConfig.DefaultCompilerPath;
            }
            else
            {
                _cachedCompilerPath = ResolveProjectPath(path);
            }
            return _cachedCompilerPath;
        }

        public static string GetCompilerPathOverride()
        {
            string envOverride = Environment.GetEnvironmentVariable("PRISM_PATH");
            if (!string.IsNullOrWhiteSpace(envOverride))
            {
                return ResolveProjectPath(envOverride);
            }

            string editorOverride = EditorPrefs.GetString(CompilerPathOverrideKey, string.Empty);
            if (!string.IsNullOrWhiteSpace(editorOverride))
            {
                return ResolveProjectPath(editorOverride);
            }

            return null;
        }

        public static string GetOutputDir()
        {
            if (_cachedOutputDir != null) return _cachedOutputDir;

            MigrateLegacyProjectIfNeeded();

            string dir = PrismProjectConfig.NormalizeOutputDir(ReadTomlValue("output_dir", "compiler"));
            _cachedOutputDir = string.IsNullOrEmpty(dir) ? PrismProjectConfig.DefaultOutputDir : dir;
            return _cachedOutputDir;
        }

        public static void ClearCache()
        {
            _cachedCompilerPath = null;
            _cachedOutputDir = null;
        }

        public static string ResolveProjectPath(string candidatePath)
        {
            return PrismProjectConfig.ResolveProjectPath(GetProjectRoot(), candidatePath);
        }

        private static string ReadTomlValue(string key, string section = null)
        {
            return ReadTomlValue(new[] { key }, section);
        }

        private static string ReadTomlValue(string[] keys, string section = null)
        {
            string filePath = GetActiveProjectFilePath();
            if (!File.Exists(filePath)) return null;

            return PrismProjectConfig.ParseTomlValue(File.ReadAllText(filePath), keys, section);
        }
    }
}
