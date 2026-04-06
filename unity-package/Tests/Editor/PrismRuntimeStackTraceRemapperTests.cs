using System.IO;
using NUnit.Framework;

namespace Prism.Editor.Tests
{
    public class PrismRuntimeStackTraceRemapperTests
    {
        [Test]
        public void TryCacheRemappedLocation_StoresAndConsumesClickableHeaderLocation()
        {
            string projectRoot = Path.Combine(Path.GetTempPath(), "PrismRuntimeStackTraceRemapperTests", Path.GetRandomFileName());
            string sourceFile = Path.Combine(projectRoot, "Assets", "Player.prsm");
            Directory.CreateDirectory(Path.GetDirectoryName(sourceFile));
            File.WriteAllText(sourceFile, "component Player : MonoBehaviour {}\n");

            try
            {
                bool cached = PrismRuntimeStackTraceRemapper.TryCacheRemappedLocation(
                    projectRoot,
                    "Assets/Player.prsm(8,10): error [PrSMRuntime] NullReferenceException: sample",
                    out string fullSourcePath);

                Assert.IsTrue(cached);
                Assert.AreEqual(sourceFile, fullSourcePath);

                bool consumed = PrismRuntimeStackTraceRemapper.TryConsumeCachedLocation(sourceFile, out int line, out int col);
                Assert.IsTrue(consumed);
                Assert.AreEqual(8, line);
                Assert.AreEqual(10, col);

                Assert.IsFalse(PrismRuntimeStackTraceRemapper.TryConsumeCachedLocation(sourceFile, out _, out _));
            }
            finally
            {
                Directory.Delete(projectRoot, true);
            }
        }

        [Test]
        public void TryResolveRemappedAssetPath_AcceptsPackagesPathsFromDiagnosticHeader()
        {
            bool resolved = PrismRuntimeStackTraceRemapper.TryResolveRemappedAssetPath(
                @"C:\PrismProject",
                "Packages/com.prsm.generated/Runtime/Player.prsm(8,10): error [PrSMRuntime] NullReferenceException: sample",
                out string assetPath);

            Assert.IsTrue(resolved);
            Assert.AreEqual("Packages/com.prsm.generated/Runtime/Player.prsm", assetPath);
        }

        [Test]
        public void TryResolveRemappedAssetPath_RejectsExternalAbsolutePaths()
        {
            bool resolved = PrismRuntimeStackTraceRemapper.TryResolveRemappedAssetPath(
                @"C:\PrismProject",
                @"D:\External\Player.prsm(8,10): error [PrSMRuntime] NullReferenceException: sample",
                out string assetPath);

            Assert.IsFalse(resolved);
            Assert.IsNull(assetPath);
        }
    }
}