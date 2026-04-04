using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;

namespace Moon.Editor.Tests
{
    public class MoonCompilerBridgeLoggingTests
    {
        [Test]
        public void LogDiagnostic_EmitsClickableErrorShape()
        {
            LogAssert.Expect(
                LogType.Error,
                "Assets/Tests/BrokenSmoke.mn(2,5): error [E050] Enum must have at least one entry");

            MoonCompilerBridge.LogDiagnostic(
                new MoonJsonDiagnostic
                {
                    code = "E050",
                    severity = "error",
                    message = "Enum must have at least one entry",
                    file = "Assets/Tests/BrokenSmoke.mn",
                    line = 2,
                    col = 5,
                });
        }

        [Test]
        public void LogDiagnostic_EmitsClickableWarningShape()
        {
            LogAssert.Expect(
                LogType.Warning,
                "Assets/Tests/Player.mn(10,3): warning [W001] Sample warning");

            MoonCompilerBridge.LogDiagnostic(
                new MoonJsonDiagnostic
                {
                    code = "W001",
                    severity = "warning",
                    message = "Sample warning",
                    file = "Assets/Tests/Player.mn",
                    line = 10,
                    col = 3,
                });
        }
    }
}