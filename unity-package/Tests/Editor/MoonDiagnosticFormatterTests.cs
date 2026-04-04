using NUnit.Framework;

namespace Moon.Editor.Tests
{
    public class MoonDiagnosticFormatterTests
    {
        [Test]
        public void GetDisplayPath_NormalizesAbsoluteProjectPath()
        {
            string projectRoot = @"C:\MoonProject";
            string fullPath = @"C:\MoonProject\Assets\Scripts\Player.mn";

            Assert.AreEqual(
                "Assets/Scripts/Player.mn",
                MoonDiagnosticFormatter.GetDisplayPath(projectRoot, fullPath));
        }

        [Test]
        public void FormatDiagnosticMessage_UsesFallbackPathWhenReportedPathMissing()
        {
            string message = MoonDiagnosticFormatter.FormatDiagnosticMessage(
                @"C:\MoonProject",
                new MoonJsonDiagnostic
                {
                    code = "E050",
                    severity = "error",
                    message = "Enum must have at least one entry",
                    file = "",
                    line = 3,
                    col = 7,
                },
                "Assets/Tests/Broken.mn");

            Assert.AreEqual(
                "Assets/Tests/Broken.mn(3,7): error [E050] Enum must have at least one entry",
                message);
        }

        [Test]
        public void FormatDiagnosticMessage_ClampsMissingCoordinatesToOne()
        {
            string message = MoonDiagnosticFormatter.FormatDiagnosticMessage(
                @"C:\MoonProject",
                new MoonJsonDiagnostic
                {
                    code = "W001",
                    severity = "warning",
                    message = "Sample warning",
                    file = "Assets/Test.mn",
                    line = 0,
                    col = 0,
                });

            Assert.AreEqual(
                "Assets/Test.mn(1,1): warning [W001] Sample warning",
                message);
        }
    }
}