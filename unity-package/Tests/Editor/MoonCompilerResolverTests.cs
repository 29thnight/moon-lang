using System.Collections.Generic;
using NUnit.Framework;

namespace Moon.Editor.Tests
{
    public class MoonCompilerResolverTests
    {
        [Test]
        public void ResolveCompilerPath_PrefersOverrideThenConfigThenDevelopmentThenBundled()
        {
            var existing = new HashSet<string>
            {
                "override/moonc.exe",
                "config/moonc.exe",
                "bundled/moonc.exe",
                "dev/moonc.exe",
            };

            string resolved = MoonCompilerResolver.ResolveCompilerPath(
                "override/moonc.exe",
                "config/moonc.exe",
                new[] { "bundled/moonc.exe" },
                new[] { "dev/moonc.exe" },
                existing.Contains);

            Assert.AreEqual("override/moonc.exe", resolved);
        }

        [Test]
        public void ResolveCompilerPath_SkipsMissingOverrideAndUsesDevelopmentBeforeBundled()
        {
            var existing = new HashSet<string>
            {
                "bundled/moonc.exe",
                "dev/moonc.exe",
            };

            string resolved = MoonCompilerResolver.ResolveCompilerPath(
                "override/missing.exe",
                "moonc",
                new[] { "bundled/moonc.exe" },
                new[] { "dev/moonc.exe" },
                existing.Contains);

            Assert.AreEqual("dev/moonc.exe", resolved);
        }

        [Test]
        public void ResolveCompilerPath_FallsBackToMooncWhenNoCandidateExists()
        {
            string resolved = MoonCompilerResolver.ResolveCompilerPath(
                "override/missing.exe",
                "config/missing.exe",
                new[] { "bundled/missing.exe" },
                new[] { "dev/missing.exe" },
                _ => false);

            Assert.AreEqual("moonc", resolved);
        }
    }
}