using System.IO;
using NUnit.Framework;

namespace Prism.Editor.Tests
{
    public class PrismProjectConfigTests
    {
        [Test]
        public void ParseTomlValue_ReadsScopedCompilerValue()
        {
            string content = @"[project]
name = ""Demo""

[compiler]
prism_path = ""tools/prism.exe""
output_dir = ""Packages/com.prsm.generated/Runtime""
";

            Assert.AreEqual(
                "Packages/com.prsm.generated/Runtime",
                PrismProjectConfig.ParseTomlValue(content, "output_dir", "compiler"));
        }

        [Test]
        public void ParseTomlValue_IgnoresMissingKeys()
        {
            string content = @"[project]
name = ""Demo""
";

            Assert.IsNull(PrismProjectConfig.ParseTomlValue(content, "output_dir", "compiler"));
        }

        [Test]
        public void ResolveProjectPath_LeavesCompilerSentinelUntouched()
        {
            Assert.AreEqual("prism", PrismProjectConfig.ResolveProjectPath("C:/PrismProject", "prism"));
        }

        [Test]
        public void ResolveProjectPath_ResolvesRelativePathsAgainstProjectRoot()
        {
            string projectRoot = Path.Combine("C:", "PrismProject");
            string resolved = PrismProjectConfig.ResolveProjectPath(projectRoot, "tools/prism.exe");

            Assert.AreEqual(
                Path.GetFullPath(Path.Combine(projectRoot, "tools/prism.exe")),
                resolved);
        }

        [Test]
        public void NormalizeProjectConfigContent_UpgradesLegacyMoonSettings()
        {
            string legacy = @"[project]
name = ""Demo""
moon_version = ""0.1.0""

[compiler]
moonc_path = ""moonc""
output_dir = ""Packages/com.moon.generated/Runtime""

[source]
include = [""Assets/**/*.mn""]
exclude = []
";

            string normalized = PrismProjectConfig.NormalizeProjectConfigContent(legacy);

            StringAssert.Contains("prsm_version = \"0.1.0\"", normalized);
            StringAssert.Contains("prism_path = \"prism\"", normalized);
            StringAssert.Contains("output_dir = \"Packages/com.prsm.generated/Runtime\"", normalized);
            StringAssert.Contains("include = [\"Assets/**/*.prsm\", \"Assets/**/*.mn\"]", normalized);
        }

        [Test]
        public void IsPrismSourceAssetPath_AcceptsLegacyAndCurrentExtensions()
        {
            Assert.That(PrismProjectConfig.IsPrismSourceAssetPath("Assets/TestScript.prsm"), Is.True);
            Assert.That(PrismProjectConfig.IsPrismSourceAssetPath("Assets/TestScript.mn"), Is.True);
            Assert.That(PrismProjectConfig.IsPrismSourceAssetPath("Assets/TestScript.cs"), Is.False);
        }
    }
}