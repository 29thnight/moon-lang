using NUnit.Framework;

namespace Prism.Editor.Tests
{
    public class PrismConsoleRemapOpenerTests
    {
        [Test]
        public void TryParseFirstPrSMLocation_PrefersDiagnosticHeader()
        {
            const string text = "Assets/TestScript.prsm(7,5): error [PrSMRuntime] DivideByZeroException: Attempted to divide by zero.\n" +
                                "[PrSM] Remapped runtime stack trace from generated PrSM C#\n" +
                                "TestScript.Awake () (at Assets/TestScript.prsm:7) [PrSM col 5]";

            bool parsed = PrismConsoleRemapOpener.TryParseFirstPrismLocation(text, out string sourcePath, out int sourceLine, out int sourceCol);

            Assert.IsTrue(parsed);
            Assert.AreEqual("Assets" + System.IO.Path.DirectorySeparatorChar + "TestScript.prsm", sourcePath);
            Assert.AreEqual(7, sourceLine);
            Assert.AreEqual(5, sourceCol);
        }

        [Test]
        public void TryParseFirstPrSMFrame_ExtractsPathLineAndColumn()
        {
            const string text = "[PrSM] Remapped runtime stack trace from generated PrSM C#\n" +
                                "DivideByZeroException: Attempted to divide by zero.\n" +
                                "TestScript.Awake () (at Assets/TestScript.prsm:15) [PrSM col 17]";

            bool parsed = PrismConsoleRemapOpener.TryParseFirstPrismFrame(text, out string sourcePath, out int sourceLine, out int sourceCol);

            Assert.IsTrue(parsed);
            Assert.AreEqual("Assets" + System.IO.Path.DirectorySeparatorChar + "TestScript.prsm", sourcePath);
            Assert.AreEqual(15, sourceLine);
            Assert.AreEqual(17, sourceCol);
        }

        [Test]
        public void TryParseFirstPrSMLocation_ParsesDotNetPrSMFrame()
        {
            const string text = "[PrSM] Remapped runtime stack trace from generated PrSM C#\n" +
                                "at TestScript.Awake() in Assets/TestScript.prsm:line 15 [PrSM col 17]";

            bool parsed = PrismConsoleRemapOpener.TryParseFirstPrismLocation(text, out string sourcePath, out int sourceLine, out int sourceCol);

            Assert.IsTrue(parsed);
            Assert.AreEqual("Assets" + System.IO.Path.DirectorySeparatorChar + "TestScript.prsm", sourcePath);
            Assert.AreEqual(15, sourceLine);
            Assert.AreEqual(17, sourceCol);
        }

        [Test]
        public void TryParseFirstPrSMFrame_ReturnsFalseWhenNoPrSMFrameExists()
        {
            bool parsed = PrismConsoleRemapOpener.TryParseFirstPrismFrame(
                "DivideByZeroException: Attempted to divide by zero.\nTestScript.Awake () (at ./Packages/com.prsm.generated/Runtime/TestScript.cs:32)",
                out _,
                out _,
                out _);

            Assert.IsFalse(parsed);
        }
    }
}