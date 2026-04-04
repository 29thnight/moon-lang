using System.IO;
using System;
using UnityEngine;
using UnityEditor;

namespace Moon.Editor
{
    /// <summary>
    /// Reads/creates .mnproject settings.
    /// Generated C# goes to Packages/com.moon.generated/ — hidden from Assets/.
    /// </summary>
    public static class MoonProjectSettings
    {
        private const string ProjectFileName = ".mnproject";
        private const string GeneratedPackageName = "com.moon.generated";
        private const string GeneratedPackageDir = "Packages/com.moon.generated";
        private const string CompilerPathOverrideKey = "Moon.CompilerPathOverride";

        private static string _cachedCompilerPath;
        private static string _cachedOutputDir;

        public static string GetProjectRoot()
        {
            return Path.GetDirectoryName(Application.dataPath);
        }

        public static string GetProjectFilePath()
        {
            return Path.Combine(GetProjectRoot(), ProjectFileName);
        }

        public static bool HasProjectFile()
        {
            return File.Exists(GetProjectFilePath());
        }

        public static void EnsureProjectFile()
        {
            if (HasProjectFile()) return;

            string projectName = Path.GetFileName(GetProjectRoot());
            string content = $@"[project]
name = ""{projectName}""
moon_version = ""0.1.0""

[compiler]
moonc_path = ""moonc""
output_dir = ""Packages/com.moon.generated/Runtime""

[source]
include = [""Assets/**/*.mn""]
exclude = []

[features]
auto_compile_on_save = true
generate_meta_files = true
pascal_case_methods = true
";
            File.WriteAllText(GetProjectFilePath(), content);
            Debug.Log($"[Moon] Created {ProjectFileName} at {GetProjectRoot()}");

            EnsureGeneratedPackage();
        }

        /// <summary>
        /// Ensures the generated code UPM package exists.
        /// </summary>
        public static void EnsureGeneratedPackage()
        {
            string root = GetProjectRoot();
            string pkgDir = Path.Combine(root, GeneratedPackageDir, "Runtime");
            string pkgJson = Path.Combine(root, GeneratedPackageDir, "package.json");
            string asmdef = Path.Combine(root, GeneratedPackageDir, "Runtime", "Moon.Generated.asmdef");

            if (!Directory.Exists(pkgDir))
                Directory.CreateDirectory(pkgDir);

            if (!File.Exists(pkgJson))
            {
                File.WriteAllText(pkgJson, @"{
  ""name"": ""com.moon.generated"",
  ""version"": ""0.0.1"",
  ""displayName"": ""Moon Generated Scripts"",
  ""description"": ""Auto-generated C# scripts from Moon (.mn) source files. Do not edit."",
  ""unity"": ""2022.3""
}
");
                Debug.Log("[Moon] Created generated package: " + GeneratedPackageDir);
            }

            if (!File.Exists(asmdef))
            {
                File.WriteAllText(asmdef, @"{
    ""name"": ""Moon.Generated"",
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

        public static string GetCompilerPath()
        {
            if (_cachedCompilerPath != null) return _cachedCompilerPath;

            string path = ReadTomlValue("moonc_path", "compiler");
            if (string.IsNullOrEmpty(path) || path == "moonc")
            {
                _cachedCompilerPath = "moonc";
            }
            else
            {
                _cachedCompilerPath = ResolveProjectPath(path);
            }
            return _cachedCompilerPath;
        }

        public static string GetCompilerPathOverride()
        {
            string envOverride = Environment.GetEnvironmentVariable("MOONC_PATH");
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

            string dir = ReadTomlValue("output_dir", "compiler");
            _cachedOutputDir = string.IsNullOrEmpty(dir) ? "Packages/com.moon.generated/Runtime" : dir;
            return _cachedOutputDir;
        }

        public static void ClearCache()
        {
            _cachedCompilerPath = null;
            _cachedOutputDir = null;
        }

        public static string ResolveProjectPath(string candidatePath)
        {
            return MoonProjectConfig.ResolveProjectPath(GetProjectRoot(), candidatePath);
        }

        private static string ReadTomlValue(string key, string section = null)
        {
            string filePath = GetProjectFilePath();
            if (!File.Exists(filePath)) return null;

            return MoonProjectConfig.ParseTomlValue(File.ReadAllText(filePath), key, section);
        }
    }
}
