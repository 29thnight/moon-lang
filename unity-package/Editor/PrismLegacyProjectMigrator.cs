using UnityEditor;

namespace Prism.Editor
{
    [InitializeOnLoad]
    public static class PrismLegacyProjectMigrator
    {
        static PrismLegacyProjectMigrator()
        {
            EditorApplication.delayCall += RunMigration;
        }

        private static void RunMigration()
        {
            bool migratedProject = PrismProjectSettings.MigrateLegacyProjectIfNeeded();
            bool movedLegacyPackage = PrismProjectSettings.MoveLegacyGeneratedPackageToBackup();
            if (!migratedProject && !movedLegacyPackage)
            {
                return;
            }

            PrismProjectSettings.ClearCache();
            PrismProjectSettings.EnsureGeneratedPackage();

            var result = PrismCompilerBridge.BuildProject();
            if (result.Success)
            {
                AssetDatabase.Refresh();
                PrismIconAssigner.AssignIconsToGeneratedScripts();
            }
            else
            {
                PrismCompilerBridge.LogDiagnostics(result);
            }
        }
    }
}