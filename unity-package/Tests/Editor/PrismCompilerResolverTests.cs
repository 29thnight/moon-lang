using System.Collections.Generic;
using NUnit.Framework;

namespace Prism.Editor.Tests
{
    public class PrismCompilerResolverTests
    {
        [Test]
        public void ResolveCompilerPath_PrefersOverrideThenConfigThenDevelopmentThenBundled()
        {
            var existing = new HashSet<string>
            {
                "override/prism.exe",
                "config/prism.exe",
                "bundled/prism.exe",
                "dev/prism.exe",
            };

            string resolved = PrismCompilerResolver.ResolveCompilerPath(
                "override/prism.exe",
                "config/prism.exe",
                new[] { "bundled/prism.exe" },
                new[] { "dev/prism.exe" },
                existing.Contains);

            Assert.AreEqual("override/prism.exe", resolved);
        }

        [Test]
        public void ResolveCompilerPath_SkipsMissingOverrideAndUsesDevelopmentBeforeBundled()
        {
            var existing = new HashSet<string>
            {
                "bundled/prism.exe",
                "dev/prism.exe",
            };

            string resolved = PrismCompilerResolver.ResolveCompilerPath(
                "override/missing.exe",
                "prism",
                new[] { "bundled/prism.exe" },
                new[] { "dev/prism.exe" },
                existing.Contains);

            Assert.AreEqual("dev/prism.exe", resolved);
        }

        [Test]
        public void ResolveCompilerPath_FallsBackToPrSMcWhenNoCandidateExists()
        {
            string resolved = PrismCompilerResolver.ResolveCompilerPath(
                "override/missing.exe",
                "config/missing.exe",
                new[] { "bundled/missing.exe" },
                new[] { "dev/missing.exe" },
                _ => false);

            Assert.AreEqual("prism", resolved);
        }
    }
}