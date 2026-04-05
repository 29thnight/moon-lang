using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;

namespace Prism.Editor.Tests
{
    public class PrismCompilerBridgeLoggingTests
    {
        [Test]
        public void LogDiagnostic_EmitsClickableErrorShape()
        {
            LogAssert.Expect(
                LogType.Error,
                "Assets/Tests/BrokenSmoke.prsm(2,5): error [E050] Enum must have at least one entry");

            PrismCompilerBridge.LogDiagnostic(
                new PrismJsonDiagnostic
                {
                    code = "E050",
                    severity = "error",
                    message = "Enum must have at least one entry",
                    file = "Assets/Tests/BrokenSmoke.prsm",
                    line = 2,
                    col = 5,
                });
        }

        [Test]
        public void LogDiagnostic_EmitsClickableWarningShape()
        {
            LogAssert.Expect(
                LogType.Warning,
                "Assets/Tests/Player.prsm(10,3): warning [W001] Sample warning");

            PrismCompilerBridge.LogDiagnostic(
                new PrismJsonDiagnostic
                {
                    code = "W001",
                    severity = "warning",
                    message = "Sample warning",
                    file = "Assets/Tests/Player.prsm",
                    line = 10,
                    col = 3,
                });
        }
    }
}