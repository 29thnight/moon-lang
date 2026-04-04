using System.IO;
using NUnit.Framework;

namespace Moon.Editor.Tests
{
    public class MoonProjectConfigTests
    {
        [Test]
        public void ParseTomlValue_ReadsScopedCompilerValue()
        {
            string content = @"[project]
name = ""Demo""

[compiler]
moonc_path = ""tools/moonc.exe""
output_dir = ""Packages/com.moon.generated/Runtime""
";

            Assert.AreEqual(
                "Packages/com.moon.generated/Runtime",
                MoonProjectConfig.ParseTomlValue(content, "output_dir", "compiler"));
        }

        [Test]
        public void ParseTomlValue_IgnoresMissingKeys()
        {
            string content = @"[project]
name = ""Demo""
";

            Assert.IsNull(MoonProjectConfig.ParseTomlValue(content, "output_dir", "compiler"));
        }

        [Test]
        public void ResolveProjectPath_LeavesCompilerSentinelUntouched()
        {
            Assert.AreEqual("moonc", MoonProjectConfig.ResolveProjectPath("C:/MoonProject", "moonc"));
        }

        [Test]
        public void ResolveProjectPath_ResolvesRelativePathsAgainstProjectRoot()
        {
            string projectRoot = Path.Combine("C:", "MoonProject");
            string resolved = MoonProjectConfig.ResolveProjectPath(projectRoot, "tools/moonc.exe");

            Assert.AreEqual(
                Path.GetFullPath(Path.Combine(projectRoot, "tools/moonc.exe")),
                resolved);
        }
    }
}