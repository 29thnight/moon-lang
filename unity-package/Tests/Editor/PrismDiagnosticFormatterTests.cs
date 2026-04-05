using NUnit.Framework;

namespace Prism.Editor.Tests
{
    public class PrismDiagnosticFormatterTests
    {
        [Test]
        public void GetDisplayPath_NormalizesAbsoluteProjectPath()
        {
            string projectRoot = @"C:\PrismProject";
            string fullPath = @"C:\PrismProject\Assets\Scripts\Player.prsm";

            Assert.AreEqual(
                "Assets/Scripts/Player.prsm",
                PrismDiagnosticFormatter.GetDisplayPath(projectRoot, fullPath));
        }

        [Test]
        public void FormatDiagnosticMessage_UsesFallbackPathWhenReportedPathMissing()
        {
            string message = PrismDiagnosticFormatter.FormatDiagnosticMessage(
                @"C:\PrismProject",
                new PrismJsonDiagnostic
                {
                    code = "E050",
                    severity = "error",
                    message = "Enum must have at least one entry",
                    file = "",
                    line = 3,
                    col = 7,
                },
                "Assets/Tests/Broken.prsm");

            Assert.AreEqual(
                "Assets/Tests/Broken.prsm(3,7): error [E050] Enum must have at least one entry",
                message);
        }

        [Test]
        public void FormatDiagnosticMessage_ClampsMissingCoordinatesToOne()
        {
            string message = PrismDiagnosticFormatter.FormatDiagnosticMessage(
                @"C:\PrismProject",
                new PrismJsonDiagnostic
                {
                    code = "W001",
                    severity = "warning",
                    message = "Sample warning",
                    file = "Assets/Test.prsm",
                    line = 0,
                    col = 0,
                });

            Assert.AreEqual(
                "Assets/Test.prsm(1,1): warning [W001] Sample warning",
                message);
        }
    }
}