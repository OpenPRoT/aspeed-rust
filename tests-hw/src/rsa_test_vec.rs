// Licensed under the Apache-2.0 license

use hex_literal::hex;

pub struct RsaKey {
    pub m: &'static [u8],
    pub d: &'static [u8],
    pub e: &'static [u8],
    pub m_bits: usize,
    pub d_bits: usize,
    pub e_bits: usize,
}

pub struct RsaTestVec {
    pub k: RsaKey,
    pub digest: &'static [u8],
    pub signature: &'static [u8],
    pub s_size: usize,
    pub d_size: usize,
}

pub static RSA_VERIFY_TV: &[RsaTestVec] = &[
    // RSA-2048 with SHA256
    RsaTestVec {
        k: RsaKey {
            m: &hex!(
                    "b21b4ae16c766bf40e21c7a80a534bf036bc258dcb2031b39e308b7addceab7c"
                    "4cf98311455a51961b978c66dd1c069d118c7fb3dd6e8c5eb4f113ee0062f034"
                    "81e342be7516b6f0a6840dbf8f1f62479b9bf75e6d9862e1f369c9b9815ae4e1"
                    "500bf9fa7403949426ad42747a6227f964a902b3a307341d6976445ef8fe25c2"
                    "8bdcebe17b364caba341eef141b9db5442ae91e761fba74401ca62cb61493642"
                    "56d85d604b085ae307aa7436a4e9f66c39c14404eab1df842914d8f7f2eda312"
                    "929a2737091564096476c693d32c1025cd5ad9150ef4294bc9c770d93d87ef80"
                    "0ad85c1fa01e76c4da3a6d3b7ae3ab45a4f182f88566b4eaae09c2b4ff3615df"
            ),
            d: &hex!(
                    "04511c23f53fb4353e1ee37a9e4c8f6f122ba9b2afe375347d4c9c5bf43b70ca"
                    "d35d75443de5a36826369c5890219ce04fef3e103edcd1bdb6588fbb197adaa4"
                    "079acb67c9779f077f0feefab4baaee476502ec4edcf89f27561385a7403bb86"
                    "a8bae8b1815574e1c97c6b57ecb8561a5d58e2736f4821dc1f4dcf6a6c1935bd"
                    "7d389a2aed446befb046ccd3f286d8f4a2c1913c915a5d9994d091d442fec6b2"
                    "89d722e1b4573841b3bea73ca32c4fd40e1bcfa7cd4d4356187f44c1c26ea37a"
                    "fee54a16df309f8233190edb4163093942491756b5da43f544f140d9392eb8f0"
                    "293f6c232d6f813f67e3a55fa19008cb768e7af7c3dcf51e878066e09c0e2781"
            ),
            e: &hex!("010001"),
            m_bits: 2048,
            d_bits: 2048,
            e_bits: 24,
        },
        digest: &hex!("990a8f23d3e56ab9f45a08894ceb937fe85abbbc3f49fdf481f744abd74fc53e"),
        signature: &hex!(
                "00e7721f180b6fbb37f13c98e84e24435def7bb7cdbf744be8d24ec2da5a895b"
                "dd4980824b1a8594fb1993458d2562166e34cfec98315f423f8a7c958c3ba881"
                "665aa7669f72ab40825dd8ee6952fa2a83a61e35741ced5c1f34a3732e8a5185"
                "bd37177535f7449e24eda75c59f1163dbb0cb30b0f475c9d588e1d47d4e0cfc7"
                "dde9c93695428f778ec393b4030d957815cfeec6b348b8a84cebcabf32c1201e"
                "61c7f4355904d648f58ebc6de6b73941c4ec0718e4f345fc7e829b7ab482eff5"
                "753cfcc347ff753bc43001d9bfd0788d2fb0b3b218f1ef9c0ee178738499dc3d"
                "025885655325e6c44a15959b43f9c3930f2f81a65dff1b7a67fef77b6d9ad1b1"
        ),
        s_size: 256,
        d_size: 32,
    },
    // RSA-2048 with SHA384
    RsaTestVec {
        k: RsaKey {
            m: &hex!(
                    "b2642b5f356ec7cab59862d4e2e294099b02be562534d78e97f03a5a0aea473d"
                    "814952832d60c0f57cee08adbde3b53b73234b09fcf7de22607063403d861ae9"
                    "c060849ff0896916f328e001a474a178b949c21b0210893d041a2d444881dd04"
                    "c98150c7bdac0d47650803cd362b463d8e90ea778de7cd46d5f0a95a44072dd7"
                    "af5f78dcf60c0eedc05120fec264e80752e812104fb5a3cef190c4a36b1e30f4"
                    "01fd8934db3f686e7d16e8e195fdc8c0e0099f529913ce1cbca0c6e38be64b75"
                    "f54d9f3a48bd756f9a3195e478563420ce23795bf29b1e4e8d771de3ba19f016"
                    "246c9c90fde7bf8407b3e259512642c8eb47ce15bee31f905628dba5a1c91579"
            ),
            d: &hex!(
                    "0fbd12ae85b24e622dff51c8e273870809eb060be7a06fdc1ebf8d0660ac26b8"
                    "e06d2da071227623b26ff5c80e56c6a88611f90eda88abca70a3b9bdf32f9fb4"
                    "3bcac828ac3a13517906842ce0eb814b7b5d6c88a1780003a816ff23651a3ffb"
                    "aa238492fac2f130f952e9b2f3328231f0200cef9b4f7626e7bd21d3dae89f3c"
                    "a05b471c90a669403871608d6fff8b558405b4a95c299702bb000b3b3b0945ff"
                    "8a6cffb746cddf64dd0dade7b371cc5b78511b188df28b69576ae90c4b3ed0cb"
                    "afd5bf6413d0241d18bb29fc1f739c5546235e6c85d10bbaf93686d07b30e8c6"
                    "670a90fadae42580d78ce905eb13e7b2a97e8ee8ca14053200408a661cd10cf9"
            ),
            e: &hex!("010001"),
            m_bits: 2048,
            d_bits: 2048,
            e_bits: 24,
        },
        digest: &hex!(
                "8863f146adde51530f947b8f93358e4c5eb522e9b0df15dd5bdf2cb00c48ea24"
                "8e838ab43194b36639ff9519491f5419"
        ),
        signature: &hex!(
                "8e1902c40981919e324e6cebc720361fc7e9eb94d34128fc8f55ac9c42445779"
                "63a47b103f3ed1b37e9e87cbd9ce364d2243abbd18748696728301911a3f2438"
                "557a275ca1b96eed09f1f8adeb20065463ebfa3dc517b6393548f53ba3e56ba9"
                "5696285e79d8f40377bc0e73868036553319d09bdc6e7e40155d922a3a0add32"
                "00f2b3b5fa313dbbe20a286a622156c90e8a1119879815373356dd68d570a306"
                "07e0bdcea0f61f440a0f7004f8bacb80ab6c6d54fcba80583adf8259e07f8d18"
                "a4586037715509a42b1c32934b6e2d92bc2894f262d535bb860e464f9ce38079"
                "a8ea8fcb895ba48e80d93dad82fd89d3c2d4e48a6799403f9b513a318b589b23"
        ),
        s_size: 256,
        d_size: 48,
    },
    // RSA-2048 with SHA512
    RsaTestVec {
        k: RsaKey {
            m: &hex!(
                    "a190b41bb223fe22193af6535038dc1fbdd62fd42783b227274eb5a1bcfb1b1c"
                    "38e4f1fdb62716c24911c0af55ca7ac7ab09cf5c0b0cf846617ae9d087b5c00f"
                    "4bd8f6f46bc6c8fa35f14fa8687f19cb0cfc56ef1497d860da4eff336f616080"
                    "03236bd94fae57373e756bda4dda78b6ef64a3f1e06b1ae2c704eaf382b13dc2"
                    "6de1aac44482a06b52aecf4aaf809f57680ef79a1954faefabaec74c035ee929"
                    "a45d1e1c5d73bd6f5776b0b9eb382108e69ee3da8902b5780536ba368fa1e620"
                    "253c7d773036c8577f6991f71e51a6abcec017b686cf2281526318a17752b129"
                    "9b2e2eb2764ce214754081a951f7f178da036c043358577e1b95085b8906c85d"
            ),
            d: &hex!(
                    "0256d027672a053d0f7a1e8f3090a4d88d778de40cef1259106fca7512818785"
                    "7394c0fa283523010cc21b34e3fc3ffb27109d20c5da2f62aece4841fd77ab55"
                    "ad5bf14b5d9f6da67864762a47da5b3b1384e314a80ad6164f8d7d92b6e5efd9"
                    "e54711d728c3ab3deb58be31d56c632606e0ebf7630539f396e7b7d26f0dcb89"
                    "40b402bfec8e601344b4100d2414387a27615bb03fd574906241e1cf2ed3e5ab"
                    "a877f28da681d0ad3249f171f5bbe88e911f03a1c23479d7032182fcd7716498"
                    "69bcdd617f39c90f8bc8ff3d0a2c2ff98cc394c308bd9d175df57de27e999167"
                    "97065478447b8728c528df6bc9bf981aa7714857b724eaf7661b85f4dc4ebec1"
            ),
            e: &hex!("010001"),
            m_bits: 2048,
            d_bits: 2048,
            e_bits: 24,
        },
        digest: &hex!(
                "db61fca7a1ac08900b84471b92c980f83dd81960026ed54a6d28aac66e48fb25"
                "2123f1414f1747d9e2820a550b24700bd49f0bde0d6b1ff3ff74eb22baf3bafb"
        ),
        signature: &hex!(
                "0ff807b0eb64f5e76899d9904f05edc29e15a550c4e9386a8352c03c0e5b162a"
                "3026e604f21503a0895cc3f75ec934193484f02bd81584faf514b21198f1671c"
                "cff45003f48a894c96e7b78765abe7f383940be20324d32b87cacea7226dbcd4"
                "3766a49e748f87b6378da253ce52a6a86cebbfdb9a2f219b420b1fb388694ce3"
                "4ee508a81b170ac2e62c820089fb7403740f07f4d78b37ee45806703dfc94c13"
                "d4bc2a300f44c97e30fd1f3b16e99bcdeed59b4db9628882078bfaa71ba1cc88"
                "0da05f083c5fe1acd7668192ef3cb00c4082a4c4ac4c14e2454941cc80068abd"
                "7f59ab8fb9a0f30e8a15a34070b4038133dcc2489097567c13da26f6c823e4a2"
        ),
        s_size: 256,
        d_size: 64,
    },
    // RSA-3072 with SHA256
    RsaTestVec {
        k: RsaKey {
            m: &hex!(
                    "9afbca2fcee254ad3c1c97764338542be611ca4e37a9cbd1296bce18acd90a25"
                    "83057b5458afaa14735911b6d82ee54836e948e4680e6b91d8aee1261380379a"
                    "2a93fc8b08af28924e0e4c5b7890dacf1025bfe410bb7688169aeee7e15ad362"
                    "809467a24a3096a2d8a5321d58942430f8bab7a1f8584841e99649b67edf9ea0"
                    "c1882a2f87450bfb6090ca155d2ce4c069f4bca9bd4e1116be8bff2c9aca26a9"
                    "82f2f70f4c7403f8a922abbb77ee8c4f63b6b7ddd92c1eabe89940ab043a1f00"
                    "439cdda02bad055c7973c0b7b7e0460fb382ea02bd5a05796b6c47a8fd81e0dd"
                    "e3a0f99b34df1b0d2baf5f58b580e33121b70702a1067b49a222b7f19429f4dd"
                    "13ec96e15c7fccd7d9dde61297cc273ad433ec745006fce9a20e0ed0016e2755"
                    "c093f87b71cc66ed8b918a2c9d9155cf3abbe784a1e0b7d2e5603c5ea2207695"
                    "ad3ebce44720da394c53456004ddbd862c02b0c50f49669b5d8e651ccaaadcab"
                    "7d0acb094f9ba7a9cf73e01945c43b40b9169f2b16789c186058e6d533066c2b"
            ),
            d: &hex!(
                    "16daf188220447e47c2c3891dcd133bdd2e1e614542a7014da523c44709e5931"
                    "604f05f9a6fd2621cf438aa9e230ed931160a4653a8b2b3088e47bee223612fd"
                    "0ddbb659c7e012259be933e2d16185edbf3ddc91da2880645ff65d2b9e3f71fe"
                    "36b9569c7240696bd3c86e1edc6f36f46f2f148c32e9774e710f695e1ba65d3d"
                    "9662143134cb93e6c5937f44589c99920938bee71bdefaaf80a3789ab0010663"
                    "e7c718c962c6ebc701cfa11a60fcb17db656131608eec95f24a00fb9fdecec4b"
                    "e2424093b09a43cc67073e7760091d8d99791a5cf5b7fdbbaeb053b8e99c7dae"
                    "7d4ec193b10c5e16e0c3f24eecb8a2d8833fb481a53803a4a56e4ee9397705ca"
                    "abe58d0844b181d77022d748bf155551c0b9ddd3b26853403d832324632e1e60"
                    "6d1d6a230008600865d7a649186fcb37abe4d8c52f222c1f53a7ec8ef1ba748a"
                    "bccbe20042a934837ea1ee47a34fb35cbd8889eed7be00d6ead3c8a5f7d29ca8"
                    "f72038ce1508b806ec8e55a2d925ec7f207ca562822a3fed6143efc6e4a2b181"
            ),
            e: &hex!("010001"),
            m_bits: 3072,
            d_bits: 3072,
            e_bits: 24,
        },
        digest: &hex!("e07a195a01492e15911eef69e788ed0233c62a205f185b115c093bf5111409fd"),
        signature: &hex!(
                "5247bd54b021c68c7ea9d8509c8a3cdc5f06859293ef37f85b5974616571839a"
                "412e8690089ef8be76ba4fb78fccafcd258dd9b546e221366bb186ceb47396c4"
                "c84cc5ab6aa8fc9d726f42df56a6feb73e568cd0c00fb2b326398355360d6a40"
                "a0327bd30b731ff1162cbffd452e07ffb6f065f163433f4af851d2de3d41d71c"
                "5ddd9d83a0b7e7042fbd3517ab39665d02650c94d549e75530e6624e393bcb4c"
                "73e9ae6fc0701bbffd1ff12c089e1a7fb9f267a061d8ec0ac41e472b367c517a"
                "7525f9de0aeeb65898bbeb9342dfc5cf42207e81b6110033aff1c961d7c625da"
                "cf009fb0a46fd7c387eb0ff9f4d46ef181a55d5244c6b69f361307b172998653"
                "671e739b169a97f467c30e0090848b8a8c3e43d75fdc532382d0609c050c887a"
                "0092620690ff1b7b8ae50b65887d682c53a57cd41994f71de754627b93fcbbc6"
                "1a75a185a0bb4e0540ab60726c827f294fc25e09a9cb231d6979e6e9eeb81baa"
                "2b269c8a7b520ca749bf75c24f6802f65d2c6760183aeef2dd5f2b805db97470"
        ),
        s_size: 384,
        d_size: 32,
    },
    // RSA-3072 with SHA384
    RsaTestVec {
        k: RsaKey {
            m: &hex!(
                    "d8ad08b52783f47941d39a32b37f318f786805634e7f62fbe754816aeead1041"
                    "ba892068b415ee368a2fc5be149b1f68c07deb8dc4c108663ec747d8a6c48c5c"
                    "fb849d21af7b2d0008c91afa50b1c15beae002b5a1f456a2b5235d30149b26bf"
                    "137cc457bfedf9c73174d209725bc39cf2365900b677b8fbdd2d95a523da8836"
                    "af5559f98c4580097e73cb65ecf7fd8491a9d6ab94fc67a45c43f04b1f2fd4e8"
                    "12fbddeb09290fc20dbb210ce27a1baae2e8eb5169c30b31ada8a7001588586d"
                    "68330b8ec9211f45e65654152f9d083a536784771db878005079d9d337f0c909"
                    "a779c6073df870ec4b374fe55bcdecb8977348ed20038b4e85f87e175c9b0e05"
                    "a0a438848b621181221e2d9f498855ea2fff71a919208185025f6732dcd3e265"
                    "dd6af448fa39c159f08ae86d983c49ac2f55b280c0380e128699ec1db4e88128"
                    "551c6fc8a89ad65cc552534d7c7f982c5517f70695918ab91106cf3cad5a2ec8"
                    "e6852230bcde5f1929b18b80552db2989a15e4757e26071efbd85d56dfb0d481"
            ),
            d: &hex!(
                    "3646fa346f06a61a2dfc34d758919737aa60dc60a90c4e1e6e2f840fe66421fb"
                    "651feea76bdd4684110fb088521f1dba1ee62c38a23dfcbda82241842b1a7752"
                    "df8ab3d5f96070c05e730a380438d308148933048ee2674570fcbfde487b2934"
                    "9a4c19df256c16b2e857ac39c394eb2688a2469f60161397bc2b2c1e6721da17"
                    "60ca8fb46c88086fd86667604c013f579329a965e7f7667b0cc5d40eed6c0cf8"
                    "69b100415d5514e6768c16d664715436f58a85502a9f3a51a27fe9fda2673a47"
                    "0f856e1a5f238809d2601196919a368b22c97893b6905b68695eb017c067e019"
                    "8de57b5b56cfecd4b1117a6f8faf76602c6fa41db2329b08a6c47627256ba680"
                    "74768a71fd553b089dcb64d8ba83e17faa902906023e40839b9762c491d15f40"
                    "c0abda6d002fe2adba323e9a0190d7362c170fcad5683743bb4e1897a02d4842"
                    "3663eb65b6bb4fa5df2b552bb3f81252304ffc061a632c5ac907411ef8aae3c4"
                    "a76b315dc679a684daea06f44f044c20e9226997d0c4447ee4cc7576dc15afd5"
            ),
            e: &hex!("010001"),
            m_bits: 3072,
            d_bits: 3072,
            e_bits: 24,
        },
        digest: &hex!(
                "045b82712bc21a00a8d44dab114dd0a2db671fc29400495d5efd47b4917de866"
                "75ba58fc99036e02111f37f5412fb5f8"
        ),
        signature: &hex!(
                "70d29ddd7d87b668b17d35575123513eed5f26f55cd38723eeee30055825dfeb"
                "7ab4fc405f6a583f8fa427e6c2f1fd8751300aa95c7c74f608e08d9b18b53790"
                "4fa1398261f16d5bfe8f422cdcd38bd4a593726283d8f631ec78ad0ce61abc52"
                "962074252b64242379fe499c719db590d9a38d4cd5b4d37b1d748da1a849e8ad"
                "bc28d64db6e6a7d11603c59f2da9ea095aa4d4f64529bdcf19d3f394721ecf28"
                "1d954152ac82485cce78eed7bedf50e2f2ef2dc15e78ffdc2f337960ee84ee50"
                "a05c7682ff4d6fc7386eb1ed97a74531240cc8f1d12927a7980a4712b1b5e60e"
                "7c29e9c5b81461edbc03acad16742ea1fceb8f32619b59ab36c294f84102a35e"
                "0f55a11b7c6654c528c621aeb4a88b8be89c8783fdc046a9e15a3caf31ca9820"
                "59273c49e0b46206a8dbb49abfc3982666e0d36c96a92a4f8973388577cf4771"
                "8d2f87557fc82ba51fe4520f494280dfc939a1883cc7a25a538fb7d3fa675e46"
                "8acdfe0d3bbf39a8ccb755a9dfb34512131c5c429c0109fe9e279d84112b4c3c"
        ),
        s_size: 384,
        d_size: 48,
    },
    // RSA-3072 with SHA512
    RsaTestVec {
        k: RsaKey {
            m: &hex!(
                    "b7f20f32d446f87e5a9c52d9c3e169577f158d6b9e2ee51eefb142b020a63a31"
                    "a59667f781d063e79a5733c04510b3b963bb8e3dfaf6d4970c62ca47610baf71"
                    "a7a52929246da24903421a6053decef3d483eed7cef67d655cc38502d2045e24"
                    "cf975b26fc6a06e3b88b7dc1342891f6c1fbcfd462ea0ce547c018eb8091a5a7"
                    "bd0eb693e9cb115d185349655b054705f450406c31f744889adf1ef409b49f01"
                    "8eeb44c976d71cbd0dea50c64b76421fb39536f7a507b38c2d582d1366e30901"
                    "8274905577a5ac53450a556d8e9f0d40ca652846d52e62a97567a12224522b20"
                    "db4a4adc874f0bd3572c302a43423c9324bb790eeb4c0fc41d319ab8af2010d2"
                    "8d45d79fd452b644d61e234443d032bb943c86254246124f16f3942e17b708cc"
                    "6f8b686422cba5e9a48c57ce0bfcb8be841973c4fe236c8d5c78fbd3b8db60ee"
                    "2a73aa37d57b4de2ea57cbde33852ebe3e8929b1251894d5285a9d035796fdea"
                    "622aa640af1bbe88bedf9ac3d05a527cb180333b68aac9483fbdc1d3f2d1d6f3"
            ),
            d: &hex!(
                    "48b6248fe11d94c8cda49193873ca9596ee0e420304d3540bf5b009382b472b8"
                    "814e2d16c777caa6187a73f882d49156ad37cc886f95f67f26e43e356d05237a"
                    "f7dbefa355c508ab77e03f70e87660b43dc3513c7991320c4958f8511f179ee7"
                    "4a2ff85636dcea9045b8ce22fb2d3655a66495c8ccb8dc141e02af5718617a6c"
                    "0dedacfa5726705cae6aedce64e67726c1669fb0a527ab2d9c9aaa2fa94e7eb4"
                    "2cfc8f1a5c21b8387b31ae705410111f5aeb96e8d9916bc5f37eb182c3dee705"
                    "c7c7cecee270d12eafd56f1a11b7441cdc1122a2e464853e1e40445bec782b23"
                    "e4f290b1d89b2b46b3ae5e52ec50afed109a5ebf7fa205b455f3d30b8db20021"
                    "e0f7b2cc739faf971799f3fc3cd09d8d0f76f1d74a3c9842db1b17a8d839432c"
                    "60638f74871a7108a0852485adb34bc9b719d24d4deb7b816caffa4fb958abb8"
                    "585ff0c78cafc7b7c07be3a41ddb1655a090fd507f837dea6be76acd9919c975"
                    "4a019a4c50c2f866215bb577b4ae661ac9dc083aacc066b4acb851327d5fddc1"
            ),
            e: &hex!("010001"),
            m_bits: 3072,
            d_bits: 3072,
            e_bits: 24,
        },
        digest: &hex!(
                "40d33dbbfdf983a086fa78a1f1495faf44db59e794066b639df2322942d70d7f"
                "1761bbc4ab0cf8fbce8354c01c0fc31fe380453817319f5846d2fc068f6c56b7"
        ),
        signature: &hex!(
                "458c7fc551357daf40db3da5246b8aa77f2961b001175700c713f7a237e43da0"
                "903f7c040d039c02d0653d03106de76a8f93cbeeaba85671cac2e1628d28aa58"
                "8b3204a0d65cbc6705c6ac955def9cafce504c3bcbd2ecf6beff59e242de4fe7"
                "7c7c92f17ca252c9146238d0455629ff31b10075559a7cecfd086323c54ceb1b"
                "e8661c4958a6cfbbffd22fb5ee5a012612a22de19d5f1020464a707bf4d5926d"
                "c7cfc7bbf3eff8b53b8dffdb9edfaf974820f0e1dab65eacefe4db5650db77c8"
                "72a5a85bca4c598dfa8b1e3549326c57a5861fd96b101fdd48c91c0f1c7a6246"
                "a401faa8584426c2bf31e80f0d2aa3f373c6fbe262a8a939838f78467a9ead15"
                "8c365aac053810ac164660f46d60c63eb9256ad71baf4707b41db88f3707d537"
                "d3385dad71ed9be6d3c2643910448f6d623dfb8a53d38c3cfb741c12acd1d30c"
                "5c21689ea27b372c8b28e9b6a3553faaa495ea2050e4dbec90dbfa21541e24a1"
                "5bb7452fc4d38c3b99bb1374c90b496d65da5d02b183ec8396d2d4e428e6b07c"
        ),
        s_size: 384,
        d_size: 64,
    },
    // RSA-4096 with SHA256
    RsaTestVec {
        k: RsaKey {
            m: &hex!(
                    "a3ca16ca9a66f1177043baef31b46ae47de39b478bbce101237b3dd190a2f072"
                    "d72a0836b99586f7bf47631989cf0145b1a912ac8c1836a24e079fc2a7650dd0"
                    "3e7bc405bad70fe03600b7ffa92205f7ed5d122314aa8c3e6b87c4a0f32b6d24"
                    "faad1abd6ef6d816796e33212c3dde05fc566f9a6193d7bfe4112f71d5403bb1"
                    "f64ef87e2acdc58df04b500a5021fd358edaabdf9dfac95a90035486491000fc"
                    "ab22cbc9b091968ec4e7de0fcbd5730b7999ff693588ebf70c6a7093b525c5be"
                    "711f0cb0f59fcaba673f719ec90efc069fb905598da5300ef87132515e0c4bd1"
                    "69cf8ac56aa1dde35906f0f3add2af9233d552ddac7747de8419e7e2769a7f38"
                    "41f831e0d571a6e7c92394ff6eb932ebfd4d7ff376574d584a92c9a45f031c41"
                    "df140881ffb1d2b77ba23152f96a9a1fb87af57ba4785972f015b23475bbfdd1"
                    "40f1a83316214ebc07cfd433ead67a7bff4c5bd216d2423480f763b18d060e9e"
                    "4aebc6a8a2af9c18a4595cf1296ec7296b420a81855fc3edf2a4d7920dc199d5"
                    "16fc91a5e87d1ed983a0ebea79ea91774301e29d49eb5774339bed2f27ea938f"
                    "69b8583809db798b4a3f14174f26ba49d6d7a6c8cba6ec558098c989222143de"
                    "07ead53d93e920b4d1124c58206e16a0c6f0411050c68ba735d6199826b3d6e4"
                    "b312670044938304394baf7aab5871e0ca5ab0a938f4e224b91a0a1b1dd62acd"
            ),
            d: &hex!(
                    "04eb977b2f1fbc0adcf5bfea1b176525a78d7c96ede0f6ee6a93337475f09b22"
                    "0c0a2874826736467f81c4bd99e8f4cc13edf5106765fbd02ce1684077f85e6d"
                    "9ccee980d58fcbdb8babc25b4508af5eb70bf67544d3e9c21f47551613b00087"
                    "9d9b3124332ee9a7652ef50cc0503c78f80f5dab68d680e2f4163b1363995a21"
                    "bab93d18472f66c9a08816a7b4a80cbbfce65f46e8af45b8b97e4c1dfb98663b"
                    "30e5eaf2cc61d740c33406f4f905d8bea5c0c54e330c41b06a785118f2d66766"
                    "14ee5e9a7b4a7953d7b1ad775c695d93525d83c660adaf6fa7b178d63e69c6a6"
                    "4c142ec246f5855da4c9435842a6c35cf43f2a8ddbb63a9bb9e2b0575a824223"
                    "9168d383f303dedf80456527c81a892fd65f9a86588f4a4a6bd169dadce3822c"
                    "499b164025a0c3c5055a210e587da0de60fa6f98b3e73a7175b70bd374791b67"
                    "26b9516fdd90a4a70f8b665a7f0918257f2d304f8122c1d3fbaa6772cbe4a817"
                    "179d22bf8dc7df6ff71696e912558ce683cc0e57511ad2a1b4336c1027d4c46c"
                    "d7224a85521d9a038e0aea90962ac435a36519e3e9d243ddda2592e00d1a6aed"
                    "de58f363d491607a7bddcfd6ce104a44551705d55f13efaf06c016a975327318"
                    "d112e311c124d4b2c58769c2426028c2948a0634a99050e13c5b1c34dedf66ff"
                    "8683b95d61683abb089507fdfee7e6227dd0e8ca4ca596951df1a734a38476c1"
            ),
            e: &hex!("010001"),
            m_bits: 4096,
            d_bits: 4096,
            e_bits: 24,
        },
        digest: &hex!("ce9f25e4b16df68c51122e23d3b4f89b804ae90d23a37f717769720f23efdd7d"),
        signature: &hex!(
                "24e1302744e833d6c5d03c7188ccbff995353f070312c0afb3d37ad5f17f13a5"
                "79fc5cb8896b185645f8fb7de480a18ed57d163b581a23b153c785ee9348e196"
                "3d5e6de7cbf486d2d4116b5c4e8a9b4df3d1fc1e758f5f1fffd2a722c24166c2"
                "7083c4a0e3434a7894a31aa376d395dd2d91386f71e85e12fd99f824a2e36125"
                "356d4ce5c8a3494cba9e1464de59a12bd973f2357c7c6e39d34fbe2162fc4d46"
                "7096215731f01a0bb6fe7e64be7b853bce4cb2ebde546987ffd500bf30ab40d1"
                "9a17460ae9f9529a4130e885ec093d6e7df7ecc444e8b25a821567078ab5aaa3"
                "7e07d437641b93954ef7647a1479f3d538779557520d3cf366954001f9710c10"
                "3823b1bd372fe6010f9ee2b432f9ec3638c0af8ff2d04be65dd386fe8f9cb694"
                "d1f3815231ec66ee4392f38a281dd887c8de48de688d8c6fc30a6a8bac6d6f7a"
                "61009e97fa776c7bb0836f7358716e38718c91a495c3d975a81de441d7f22085"
                "13995a1ab2a93480e20ff0c54e6c213ff9f5473b2d650372e2eac034a4f58748"
                "84ebdcca6d57d9975cfe5b185ae82966bcbff3e65d4df734f550cac2cca8403b"
                "a941efc44d792165602c27220fb42894e5ca68532e17312772952d1166b56ab0"
                "c2a2f92c99450b5cf89be92c1b588371a122ff8407cd0930a49ead3cdac40c68"
                "10c60183ae3dbdd0dae031dd2b32ebc5fc9230de46835caf728a6df00ed5ae03"
        ),
        s_size: 512,
        d_size: 32,
    },
    // RSA-4096 with SHA384
    RsaTestVec {
        k: RsaKey {
            m: &hex!(
                    "b9a6ded4e8faa38d929ce59c0290126730438920dc7b4091bd85283b164d4ac4"
                    "bdc850fe4d9e51fa067a3d8150fe15b593c6ec4dda41a813d1dd9f0313ac11f5"
                    "69321901f0da0ec1bb942205f03b0e1e756734e6a6baf76109bb2cd83008c656"
                    "7c79f9106dece48552fab21fd879f5d3a133f5f3e29b4eae7afceacd9f00c25d"
                    "60443a07984d52a71c98a3672fc22469e52b5dabe1c6554d12f10d536604536e"
                    "db7d53027b01bdd1a12ecc595621672f2c4a79147c9594ac71c5f76b8eca3f67"
                    "088a1ea9e66801a1379a04f03b785053f119b3b38cae1dd812576b0f57903a3e"
                    "a791bc86c1c264529533e8af4661fcc0184db26c7e3950afae432aaa34fd2742"
                    "3032a4adbeea5d0f464c0bc7ef1d922b682111b287e79b125f5ca9ed6fe7faea"
                    "3a009ea3ad3f819e46bfc1b36fbbb85d85d9c906402d38188808bfa672e8205e"
                    "7fdd75caff0f5fbf1a7319d1f28798ebd292ce1c2c307753b4f49a4d16b261f4"
                    "0fecd20a7fdc4e4beaec39477f2f97a083e7d419c8ecdee98a0e21cfd3f978ff"
                    "ef59301430d97954c46016715d4c33b0d6a4d11cd2fe1a1c038ace3d708d7aa8"
                    "f9130371dc5e5e908e48f34cff9ea4685958c62d37c7df49a214d61dd7257fd7"
                    "3365d98d0752f527fe8aaa2eaa2dd3df410c1b5861b256bd063a88f06867a9ad"
                    "1b950440c6f5b111d6069a549b586e916d624c961c7398c1e091cb1bb130012f"
            ),
            d: &hex!(
                    "217df02f3ac494a2e2b3395cf6bd6504869978709c36040506cab1e24ef8aca7"
                    "67916ec321df90ec6924be59c8f1bd9adf4900b25ed07f70ebe86e5a50bf3712"
                    "2de322cdcdd498daec6b21e42e5d890387ac91c796869e403c502f1be5368009"
                    "743550f05cd462b986a9c15eab5ffd302ba1c31a37644b4280df85c08891cb7e"
                    "aaf24401c0c11a4b198693dbf6aab0850619d7b13d6ed94ea93f21674d3d348c"
                    "083ec8ade6e0fe8c85e67a691777b934b269d1041821a80ba00ed5beeaa8a3dd"
                    "8395df48494fb413908351a6c123c8620c59d5a86b95b1be9c387710f27c0b75"
                    "e8270b7ac813b8a389552f0207d17f966c0c0047405238b328dde72183a3b3e4"
                    "173fe7c4e15948e4d8e94c68da12df87073bab68b75596391399a4f298f5619c"
                    "969ceaf592a36a41e7864bf4e51eb326c6ff27bb8d468044a17729e69db3e3e4"
                    "859ad92493121e90259f2ac79f94a356f88dee33e6c87f1de9e272d4e7fad796"
                    "b0671c8bf65c1b42f9e2efec4cd0100863fe58973e75b98db25d8df53d7494ec"
                    "a21546b6b8e6a3b472fef144b7633b09a8b83286d2955cafa4fb9625e578122d"
                    "abdc0783c05c19d7e77339a77508d19431ce3c76bb7e9c6bfe9e85abf2c87a67"
                    "6c8b1aa88a1697a7b02dabc5bde9beeb34fa3c908a9718c42c53b12508c6b0b8"
                    "ab94e7a76c4a534e2646d1076da56dcb8cb5231b2211ded5dccf8bba63e57241"
            ),
            e: &hex!("010001"),
            m_bits: 4096,
            d_bits: 4096,
            e_bits: 24,
        },
        digest: &hex!(
                "3d2aec83993ad22023c983b574f20cc7c041c7df11b9e9b058ff06e1ad1331cb"
                "7e92c512317c015a48d35b997482deb8"
        ),
        signature: &hex!(
                "89356ddce1f2cafa679bcb3814eb1b6103374ca2c16da773da8c5bbed65cb7cd"
                "f29d5786ac598bb5f4413cccb795044cc1a4aa1a59ccc002b132ee968c756fd8"
                "eb86b748f2a89a3f633cb10e88914263ab2a1915d5018d41f78da448057274bf"
                "73a0882e11b288f6549b4f5cf9203af625e70c5109e497c018bdef233fa67c4d"
                "2770b88d103e106cb6826ac3aabba73da19ecba0f26a306cc286169a1b5d799c"
                "2dbb6462a1f3d2838014e9227b3f48638c27aa6f55bcff640319cea1b8b10937"
                "460b8d0901b1e2d26d597f50def64c2acf9826b19d0ce990674a3904075da5ea"
                "59bec4decd152dc878c0d102d80af6fe87a40c007f18ceecbba6c320160d89e4"
                "18d68f954399c4235ef04920337fc87e0ccd442766abd789f682e1e21db4b9d0"
                "6185223ec0fcd202ffe4529bcaec0e61e2a5e538e1aa65e1f402e6d60962cd79"
                "ff29649bfab9ca6ac0f293ce050eb2fee4ade30704726f2e1ddbdb6a3bdc5ed3"
                "6c48418d5b84168bb2fc053a8a6dda4deeba96a59a6235369b983088439271e9"
                "3a7603544fb5983fc031e7bed3996efc9a83322a3bdd74edc35837fbed9d28e0"
                "818aef79ec4481429a0ba2cd9919a9e74db9bfc58e32642e9352f43e4ffb520e"
                "cf085e32dbc25a3acf56f9351fdffc0b874219fac5ff3f3335555de42e38ee52"
                "32ad99c1f7b7f2de316c2384c1191bd61bb1c7b1e3b9a3bdc980bb187a1997c5"
        ),
        s_size: 512,
        d_size: 48,
    },
    // RSA-4096 with SHA512
    RsaTestVec {
        k: RsaKey {
            m: &hex!(
                    "c0050a48abb0b53bb7876069f655ad57b084ff44e4d0e88800bff90c4681eb93"
                    "1f06e924f125649c4df5b0dd33b9eb82f9c83da85a1bb00acbc79120b20c07ec"
                    "918a622337955f603c3bf0bbd4e8c0f941f9c56d7b6bccf25ff97acfae18ff64"
                    "4daf60a3e6aff71640ad198b6ed53aba90e4f50c5847d95d748fc800bf9534e3"
                    "53ae6f0568f71f65c11145a30cd26d29fe049dbe800ff3bc08860737f17d653f"
                    "15400ede9fd8ff2f3db3221fdc302c92862eba098a97b2973b10df56ab4f0d97"
                    "4231910808f67503b859130d02c49569c6c332983b90005e53fe9cde0f61ad16"
                    "2c1515eec43e9bf15ee55ecc5d0e6aca630c8fddd7e799557775891b8e9b6318"
                    "4facce1a82822e03cadcbb981d77a160b069b5bb4553e74756b1752e16fe5deb"
                    "4cc81a4881c229edfef0633fb0306dbac9341fe5c6817d0eb8263ed69955f9db"
                    "735f370fd09f302be9ec505332e70ce009c23c3160bd92e739b41cb6571902da"
                    "74a6bd0d7793ce5c10a5475ea07acc971fe1731b11ad30d15f66731d59b25d52"
                    "707b8d175752c1676ea3f2fc6f7f1a7794d9f3c15e9f21ae3e6739e6539b0012"
                    "e0dd75ec810c4f66f0dc487013f6aa3edf4810b779069b205c9b02cd22e640db"
                    "810f3c247ba83e6d4ee82c94218e5d5cd0628db0231846e968afecdb17e17634"
                    "322ca2805cb9344c13c5e81f2b2fb0427a258468a59282d36bc869c73b784823"
            ),
            d: &hex!(
                    "062b4348d16744734e32a5b8d1c6afe27cd3d105ac1a40163a7bf06ae20b037f"
                    "ff03b052d50280087b82b4b36e997f04feb45db2be6e5c4a0480fd5b210d3ce5"
                    "ef49145d104896bf57400fbc611b3733ead834ed0f0588189357c707cc0e9689"
                    "e8eb04fe4f078137304dbf5fd8e835d24f9c6379098f095f34c2f7c87a5f374b"
                    "44617ac4a03a2c428f20d86d64189094444a6eacf39d4f95cf6a4aea13b29521"
                    "a133a526a6d9085dc2ac3220f6e87f4e17cc818293f6cab08c1fbf25202cd281"
                    "750975a0ddf4d90e8d0a23900bf237e0a59798d50638daa22d2da56986f1fae7"
                    "16f478776acb9e23f38a07f125de34351b588878b69a84b517b8e9763eaf2450"
                    "51e3a1bbf7193c245371bf67becf8c5dfda94a7813c3cb01964bd85806f554a4"
                    "17f4f8338cdd92160c123ae58b57047ff2198c2425cd793cac77900cbb312410"
                    "4c7cae4382b4f9de0f47c977b4a2f5b211cb7740feb96e54de131ce2d76255b4"
                    "8ab8aba8cd0402e9c20ffb3d317ee645449189f76426ad353ed28d11d1b370c9"
                    "c2a9283c3853a3caf1575301e321efcfab9b504c6f93240678a896d80e3c57ae"
                    "e09410cdb33a7f2d152dd38be97922f7a86f553d69e003834d6869d1da5d42c0"
                    "8a9407fd180bf8161e3d09ef2cef555736d7928276e4ebfa5e76692ca902ff2b"
                    "1bdf294a6c8a30e40c4a2d9c9f1a31dd1151703cb326d38e7389c03e4691b96d"
            ),
            e: &hex!("010001"),
            m_bits: 4096,
            d_bits: 4096,
            e_bits: 24,
        },
        digest: &hex!(
                "591ebc7d38c0ba5c3992f6a092d1434546e9b13b967b30eeb77e81d33e5e8674"
                "8265cc6ad9c925bae1faa8aa2dc51c584b3cf4b06c4c60bd19163fe616f606aa"
        ),
        signature: &hex!(
                "2c4b1d7bebee385dc0e0b3252afa4e2b08624301b84a218880cb565bff6f5e09"
                "0f3debce83c79bed45b036df7a34ed5a175cb0fe8ab95bd46a9a9c2ce78a0482"
                "af9ab514c840c10ed564ccddd833a1a4516f98da1ff4ec7f6e3c8ccfb758018d"
                "56112009a0a0bc17c40d97119ffcc00283ebf34e0c3eafedf61057bfb43d26a8"
                "26390ad2911c4ee8fdbdf2565ac92939c339c37887f085799eb18a1a24e9cb75"
                "05fb29ecd08235714b46f63681169f8222dc3a48b46421c0ecc6d75c58566558"
                "5f21ef4dc460ce8faa114dbc493b1f363316f8811e5359f2ecf6d48387955955"
                "a5a13b1b1960544766d9b4aa7cb197cbc01b250e960594dbdb37461a5c5c3769"
                "a8f08c623a1cbd617d1b96c98d9add5ad5031eeb8bd2b139dbc420dbbdc47c8f"
                "b019df72ef2c643d9f6625f48b4df55b2979d228a5af595de28a389521d30685"
                "c2dc181c532bc8bf92a0d58c49ff73ed4c318c88a3c3e2b376662f501f62128a"
                "d81e1099dbe347915c8d578f058e3fb267916d944496e892ebcb6480bc8b7466"
                "0a38c5147cf3271b2bfe772b9da308e7c190b23e9c6af09ed3c5a13f80b31625"
                "25506348c40d6cfdd69588e8f12bf6a5cc8930bb289e9b18e59adedd2abff2fc"
                "ad365f27bb45c5d7b1b35fcf96ab1087b4b419c99d142df99a184fee1dfea1df"
                "5192f6a59ef9540758b57e8064092764a3c2b696c58e117cb5900a6c0789f318"
        ),
        s_size: 512,
        d_size: 64,
    },
];
