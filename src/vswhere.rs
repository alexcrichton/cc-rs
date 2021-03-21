use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::io::BufRead;
use std::path::PathBuf;

use crate::setup_config::{EnumSetupInstances, SetupInstance};

pub enum VsInstance {
    Com(SetupInstance),
    Vswhere(VswhereInstance),
}

impl VsInstance {
    pub fn installation_name(&self) -> Option<Cow<str>> {
        match self {
            Self::Com(s) => s
                .installation_name()
                .ok()
                .and_then(|s| s.into_string().ok())
                .map(Cow::from),
            Self::Vswhere(v) => v.map.get("installationName").map(Cow::from),
        }
    }

    pub fn installation_path(&self) -> Option<PathBuf> {
        match self {
            Self::Com(s) => s.installation_path().ok().map(PathBuf::from),
            Self::Vswhere(v) => v.map.get("installationPath").map(PathBuf::from),
        }
    }

    pub fn installation_version(&self) -> Option<Cow<str>> {
        match self {
            Self::Com(s) => s
                .installation_version()
                .ok()
                .and_then(|s| s.into_string().ok())
                .map(Cow::from),
            Self::Vswhere(v) => v.map.get("installationVersion").map(Cow::from),
        }
    }
}

pub enum VsInstances {
    ComBased(EnumSetupInstances),
    VswhereBased(VswhereInstances),
}

impl IntoIterator for VsInstances {
    type Item = VsInstance;
    #[allow(bare_trait_objects)]
    type IntoIter = Box<Iterator<Item = Self::Item>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            VsInstances::ComBased(e) => {
                Box::new(e.into_iter().filter_map(Result::ok).map(VsInstance::Com))
            }
            VsInstances::VswhereBased(v) => Box::new(v.0.into_iter().map(VsInstance::Vswhere)),
        }
    }
}

pub struct VswhereInstances(Vec<VswhereInstance>);

impl<'a> From<&Vec<u8>> for VswhereInstances {
    fn from(output: &Vec<u8>) -> Self {
        let lines: Vec<_> = output.lines().filter_map(Result::ok).collect();

        let instances = lines
            .split(|s| s.is_empty())
            .filter_map(|s| VswhereInstance::try_from(s).ok())
            .collect();

        Self(instances)
    }
}

#[derive(Debug)]
pub struct VswhereInstance {
    map: HashMap<String, String>,
}

impl TryFrom<&[String]> for VswhereInstance {
    type Error = i32;

    fn try_from(lines: &[String]) -> Result<Self, Self::Error> {
        let map: HashMap<_, _> = lines
            .iter()
            .filter_map(|s| {
                let mut splitn = s.splitn(2, ": ");
                Some((splitn.next()?.to_owned(), splitn.next()?.to_owned()))
            })
            .collect();

        if !map.contains_key("installationName")
            || !map.contains_key("installationPath")
            || !map.contains_key("installationVersion")
        {
            return Err(2);
        }

        Ok(Self { map })
    }
}

#[cfg(test)]
mod tests_ {
    use std::borrow::Cow;
    use std::path::PathBuf;

    #[test]
    fn it_parses_vswhere_output_correctly() {
        let output = br"instanceId: 58104422
installDate: 21/02/2021 21:50:33
installationName: VisualStudio/16.9.2+31112.23
installationPath: C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools
installationVersion: 16.9.31112.23
productId: Microsoft.VisualStudio.Product.BuildTools
productPath: C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\Common7\Tools\LaunchDevCmd.bat
state: 4294967295
isComplete: 1
isLaunchable: 1
isPrerelease: 0
isRebootRequired: 0
displayName: Visual Studio Build Tools 2019
description: The Visual Studio Build Tools allows you to build native and managed MSBuild-based applications without requiring the Visual Studio IDE. There are options to install the Visual C++ compilers and libraries, MFC, ATL, and C++/CLI support.
channelId: VisualStudio.16.Release
channelUri: https://aka.ms/vs/16/release/channel
enginePath: C:\Program Files (x86)\Microsoft Visual Studio\Installer\resources\app\ServiceHub\Services\Microsoft.VisualStudio.Setup.Service
releaseNotes: https://docs.microsoft.com/en-us/visualstudio/releases/2019/release-notes-v16.9#16.9.2
thirdPartyNotices: https://go.microsoft.com/fwlink/?LinkId=660909
updateDate: 2021-03-17T21:16:46.5963702Z
catalog_buildBranch: d16.9
catalog_buildVersion: 16.9.31112.23
catalog_id: VisualStudio/16.9.2+31112.23
catalog_localBuild: build-lab
catalog_manifestName: VisualStudio
catalog_manifestType: installer
catalog_productDisplayVersion: 16.9.2
catalog_productLine: Dev16
catalog_productLineVersion: 2019
catalog_productMilestone: RTW
catalog_productMilestoneIsPreRelease: False
catalog_productName: Visual Studio
catalog_productPatchVersion: 2
catalog_productPreReleaseMilestoneSuffix: 1.0
catalog_productSemanticVersion: 16.9.2+31112.23
catalog_requiredEngineVersion: 2.9.3365.38425
properties_campaignId: 156063665.1613940062
properties_channelManifestId: VisualStudio.16.Release/16.9.2+31112.23
properties_nickname: 
properties_setupEngineFilePath: C:\Program Files (x86)\Microsoft Visual Studio\Installer\vs_installershell.exe
"
        .to_vec();

        let mut vswhere_instances = super::VswhereInstances::from(&output);
        assert_eq!(vswhere_instances.0.len(), 1);

        let vs_instance = super::VsInstance::Vswhere(vswhere_instances.0.pop().unwrap());
        assert_eq!(
            vs_instance.installation_name(),
            Some(Cow::from("VisualStudio/16.9.2+31112.23"))
        );
        assert_eq!(
            vs_instance.installation_path(),
            Some(PathBuf::from(
                r"C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools"
            ))
        );
        assert_eq!(
            vs_instance.installation_version(),
            Some(Cow::from("16.9.31112.23"))
        );
    }

    #[test]
    fn it_returns_empty_list_for_empty_output() {
        let output = b"".to_vec();

        let vswhere_instances = super::VswhereInstances::from(&output);

        assert_eq!(vswhere_instances.0.len(), 0)
    }

    #[test]
    fn it_returns_empty_list_for_output_consisting_of_empty_lines() {
        let output = br"

"
        .to_vec();

        let vswhere_instances = super::VswhereInstances::from(&output);

        assert_eq!(vswhere_instances.0.len(), 0);
    }
}
