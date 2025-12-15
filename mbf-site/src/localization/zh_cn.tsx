import { Eng } from "./en";

export const SimplifiedChinese = {
    __proto__: Eng, // use english text for missing items

    sourceCode: <>源码</>,
    titleText: <>想在Quest上为节奏光剑安装自制谱面，这是最简单的工具!</>,

    toGetStart:
        <>
            <p>若要开始，使用USB-C数据线连接Quest设备，并点击下方按钮。</p>
            <p>想看看有哪些能用的模组？可以在<a href="https://mods.bsquest.xyz" target="_blank" rel="noopener noreferrer">这里</a>找到完整的清单！</p>
        </>
    ,

    notInstalled: <>
        <h1>没有安装节奏光剑</h1>
        <p>请在商店中安装节奏光剑，然后刷新此网页。</p>
        <h3>你感觉自己已经安装了吗？</h3>
        <p>偶尔会有这种情况，在头显里看起来已经安装了游戏，但实际上并没有（是Meta的软件Bug）。</p>
        <p>可以这样修复：在Quest上进入<b>资源库</b>，然后点击节奏光剑旁边的三个点，选择<b>卸载。最后在商店里重新安装节奏光剑，并刷新此网页重试。</b>。</p>
    </>,

    noInternet: <>
        <h1>没有网络</h1>
        <p>你的<b>Quest设备</b>没有互联网连接。</p>
        <p>为了给游戏打补丁，MBF需要下载一些重要文件，比如模组加载器和一些重要的模组。
            <br />这需要你的Quest能联网。请确保WiFi已经开启，然后刷新此页。</p>
        <p><b>请注意，Quest设备需要能够访问Github等国际互联网服务！</b></p>
    </>,

    noCompatableDevice:<>
        <h3>没有兼容的设备？</h3>
    
        <p>
          若要使用MBF，你必须启用开发者模式，这样才能用USB连接你的Quest设备。
          <br />参考<a href="https://developer.oculus.com/documentation/native/android/mobile-device-setup/?locale=zh_CN" target="_blank" rel="noopener noreferrer">官方指南</a> -
          你需要创建一个新的组织，然后启用USB调试。
        </p>
    </>,
    connectToQuest:<>连接到Quest设备</>,


    modCompatable: <>你的节奏光剑已经注入模组了，并且当前版本也兼容模组。</>,
    modUpdateAvaliable: <>然而，现在有一个新版本的游戏也能用模组了！ </>,

    clickHereToUpdate: <>点击这里更新</>,
    updateBeatSaberHint: <>
        <h2>更新节奏光剑</h2>
        <p>若想更新至最新的可用模组的版本，只要：</p>
        <ol>
            <li>按下面的按钮卸载节奏光剑。</li>
            <li>在头显里面重新安装节奏光剑。</li>
            <li>回到MBF，给你刚刚装的游戏版本注入模组。</li>
        </ol>
    </>,

    uninstallBeatSaber: <>
        卸载节奏光剑
    </>,
    cancel: <>取消</>,

    uninstallAboutMapThings: <>
        <h3>我的谱面/模组/成绩/qosmetics会怎么样？</h3>
        <ul>
            <li><em>谱面和成绩是安全的</em>，因为它们存放的位置和游戏本体是不一样的，你卸载的时候不会自动删掉它们。</li>
            <li>Qosmetics也不会被删除，但是如果新版本不支持qosmetics的模组，那就无法使用了。如果你哪天想念它们了，还可以随时回退到这个版本。</li>
            <li><em>现在所有已经安装的模组都会被删除。</em> （会自动安装更新后游戏版本上的核心模组）你可以在更新游戏后重新安装自己的模组（如果新版本有这些模组的话）。</li>
        </ul>
    </>,

    loadingDowngradedApk: <>
        <h2>加载降级APK清单文件</h2>
        <p>这通常会持续几秒钟。</p>
    </>,

    modWarning: <>
        <h2 className='warning'>请仔细阅读！</h2>
        <p>模组和自制谱面并不是由Beat Games官方支持提供的。你可能会遇到原版游戏没有的Bug和崩溃。</p>
    </>,

    permissions: <>权限</>,

    modTheApp: <>开始补丁</>,
    changePermissionHint: <>
        <h2>修改权限</h2>
        <p>某些模组需要让节奏光剑的app拥有特定安卓权限才能正常工作。</p>
        <p>（也可以稍后修改权限，所以如果不知道要用啥，没必要现在全都打开）</p>
    </>,

    confirmPermission: <>
        确认权限
    </>,

    appPatchedHint: <>
        <h1>补丁正在进行</h1>
        <p>这通常会持续几分钟，但如果你网络不太好，也可能会更久。</p>
    </>,
    dontDisconnectDeviceHint: <>在此过程中请勿让设备断开连接。</>,
    chooseDifferentGameVersionHint: <>
        <h2>选择一个不同的游戏版本</h2>
        <p>在这个菜单中，你可以让MBF降级至非最新可用模组版本的游戏</p>
        <p>这并不推荐，除非你真的想玩一个模组，但它在最新版本游戏上还没有，才要这样做。</p>
        <p><b>请注意MBF不支持将游戏降级至1.28及更低版本。</b></p>
        <p>点击一个版本号然后开始降级：</p>
    </>,
    useLatestModdable: <>使用最新的模组可用版本</>,

    confirmDowngrade: <>确认降级</>,
    versionSupportedMessageTitle: <>安装自制谱面</>,
    versionSupportedHint(version:string) {
        return <>你的游戏现在版本号是{version}，支持模组！</>
    },

    chooseAnotherVersion: <>（选择另一个版本）</>,


    settings:<>设置</>,
    credits:<>致谢</>,

    showAnimatedBackground:<>显示背景动画</>,

    Logs:<>日志</>,

    allowConnectionInHeadSet:<>
        <h2>在头显设备中允许连接</h2>
        <p>戴上你的头显，然后点击<b>“始终对这台电脑允许”</b></p>
        <p>（只需要做这一次就可以。）</p>
        <h4>没有看到提示框？</h4>
        <ol>
        <li>刷新当前网页。</li>
        <li>戴上你的头显，<b>一定要保持戴在头上</b>。</li>
        <li>再次试着连接Quest设备。</li>
        </ol>
        <p>（有些时候Quest只会在设备处于佩戴状态时，才会弹出提示。）</p>
        <p>如果还是不行，<b>重启Quest设备再试一次。</b></p>
    </>,


    creditsIntro:(SourceUrl:string)=><>
        <p>你好，我是<b>Lauriethefish</b>，ModsBeforeFriday的原作者。</p>
        <p>MBF是一个<a href={SourceUrl}>开源项目</a>，在开发过程中，有很多人对其进行了改进。</p>
        <p>请记住一件重要的事情，MBF只是在帮你<em>安装</em>模组。在这些被安装的核心模组的背后，有一群非常天才的人们，
        如果你不仔细看模组列表，你甚至都不会知道他（她）们的名字！</p>
        <p>此菜单仅关注那些对MBF应用本身做出贡献的人。</p>
    </>,
    mbfContributors:<>MBF贡献者</>,

    contributorIntroFrozenAlex:<>制作了MBF的拖拽系统， 为我在UI设计方便提供了很多启发。如果没有他，这个UI会乱七八糟的！ </>,
    contributorXoToM:<>，是我朋友，制作了这个令人喜爱的动画背景。（虽说你的CPU可能会讨厌这个东西！）</>,
    contributorAltyFox:<>，是BSMG支持团队中的一个人，为可用性方面提供了宝贵的反馈，帮我定位并修复了Bug。</>,

    contributorLocalization:<>{/* nothing here, place holder for localization translators*/}</>,

    creditsOkBtnText:<>确定</>,

    checkInstall: <>正在检查节奏光剑的安装情况</>,
    mightTakeFewTimes: <>这可能会花费几秒钟，第一次会更久。</>,

    appIsModded: <>游戏已经注入过模组</>,

    updateAndSetupMods: <>升级游戏并注入模组</>,
    downgradeAndSetupMods: <>降级游戏并注入模组</>,
    doesntSupportMods: <>MBF检测到你现在的游戏版本还不支持模组！</>,
    canDowngrateToVersion: (version:string)=>
        <>幸运的是，此版本可降级至最近的支持模组的版本：{version}</>
    ,

    EditXML:<>编辑XML</>,
    SimpleOptions:<>简单选项</>,
    AdvancedOptions:<>高级选项</>,

    downloadCurrentXML:<>下载当前XML</>,
    uploadXML:<>上传XML</>,
    backBtnText:<>返回</>,

    permMicrophone:<>访问麦克风</>,
    permPassthrough:<>访问透视摄像头</>,
    permBody:<>身体追踪</>,
    permHand:<>头部追踪</>,
    permBluetooth:<>蓝牙</>,
    permMRC:<>MRC（混合现实捕捉）环境</>,

    deviceInUse:<>设备正在被占用</>,
    failedToConnectDevice:<>连接设备失败</>,

    otherAppIsAccessQuest: <>一些其它应用正在访问你的Quest设备，比如SideQuest之类的。</>,

    killAdb:<>
        <p>如果要修复这个问题，关掉SideQuest（如果你打开了的话），然后按<span className="codeBox">Win + R</span>键并输入下面的内容，接着按回车。</p>
        <span className="codeBox">taskkill /IM adb.exe /F</span>  
        <p>或者也可以重启电脑。</p>
    </>,

    askLaurie:<><p>如果还是不行的话，请在Discord联系<code>Lauriethefish</code>来获得支持。我们正在努力适配连接/驱动问题，
    这需要<i>你的帮助！</i></p></>,

    failedToFetchManifestHint:"无法获得用于版本降级的AndroidManifest.xml文件。是不是Quest设备突然断网了？",

    modDevelopmentWarn:<>模组开发模式已启用：跳过版本检查。此模式仅供开发者使用！</>,

    everythingReady:<>一切就绪！</>,
    notSureNext:<>不知道接下来做什么？</>,

    nextSteps: <ul>
        <li>打开游戏看看左边。会有一个菜单，显示了你的模组。</li>
        <li>点击<b>"SongDownloader"</b>模组，然后在游戏里寻找自制歌曲。</li>
        <li>看看下面这些模组，下载它们！</li>
    </ul>,
    yourMods:<>你的模组</>,
    addMods:<>添加模组</>,
    uploadFiles:<>上传文件</>,

        installModHint:(hasUpdate:boolean, hasNewMod:boolean, modCount:number)=><>
        {hasUpdate && hasNewMod && "安装或升级 "}
        {hasUpdate && !hasNewMod && "升级 "}
        {!hasUpdate && hasNewMod && "安装 "}
        {modCount} 个模组
    </>,
    updateBtnText:<>升级</>,
    installBtnText:<>安装</>,
    sourceCodeBtnText:<>源码</>,
    reportBugBtnText:<>报告Bug</>,

    coreBadgeText:<>核心</>,

    optionMenuModTools:<>模组工具</>,
    optionMenuAdbLog:<>ADB日志</>,
    optionMenuChangePerm:<>修改权限/重新补丁</>,

    optKillBeatSaber:<>关闭节奏光剑</>,
    optKillBeatSaberDesc:<>立即关闭游戏。</>,
    beatsaberKilled:<>已成功关闭游戏</>,
    optRestartBeatSaber:<>重启节奏光剑</>,
    optRestartBeatSaberDesc:<>立即关闭并重启游戏。</>,
    beatsaberRestarted:<>已成功重启游戏</>,
    optReinstallCore:<>重新安装至仅核心模组</>,
    optReinstallCoreDesc:<>删除所有已安装的模组模组，并只重新安装核心模组。</>,
    reinstallOnlyCoreMods:"重新安装至仅核心模组",
    failedToReinstallOnlyCoreMods:"重新安装（仅）核心模组失败",
    allNonCoreRemoved:<>所有的非核心模组已经被移除！</>,

    optUninstallBeatsaber:<>卸载节奏光剑</>,
    optUninstallBeatsaberDesc:<>卸载游戏：这会移除所有模组，然后关闭MBF。</>,

    optFixPlayerData:<>修复玩家数据（Player Data）</>,
    optFixPlayerDataDesc:<>修复一个由玩家数据权限导致的问题。</>,
    optFixPlayerDataSuccess:<>成功修复玩家数据问题</>,
    optFixPlayerDataNoData:<>没有找到需要修复的玩家数据文件</>,

    failedToKillBeatsaber:"游戏进程kill失败",
    failedToUninstall:"卸载游戏失败",
    failedToFixPlayerData:"玩家数据修复失败",

    author_by:<>作者 </>,

    changePermHintInOptionsMenu:<>
        <p>某些模组需要开启特定的安卓权限才能工作。
            为了开启权限，你需要重新补丁游戏，可以通过下面的按钮自动操作。</p>
    </>,

    repatchGame:<>重新补丁游戏</>,

    optionsMenuAdbLogHint:<>
        <p>此特性能让你获取一个日志，以查明Quest设备中发生了什么事情，可以用于让模组开发者修复bug。</p>
        <p>点击下面的按钮，<span className="warning">让头显保持有线连接状态。</span> 打开游戏做些什么事情，触发你的bug或者问题，然后再次点击这个按钮。</p>
    </>,

    startLogging:<>开始记录日志</>,
    stopLogging:<>停止记录日志</>,
    waitingForLog:<>请等待，正在生成日志……</>,
    downloadLog:<>下载日志</>,

    selectSplashScreen:<>选择开屏画面</>,
    usingSplash:(name:string)=><>（正在使用<code className="codeBox">{name}</code>作为开屏画面）</>,

    logOutput:<>日志输出</>,


    fixWithRestartDevice:(isViewingOnMobile:boolean)=>
        <>
        若要修复此问题，重启你的{isViewingOnMobile ? "手机" : "电脑"}.
        </>
    ,

    questBrowserMessage:<>
        <h1>已检测到Quest浏览器</h1>
        <p>MBF发现你正在使用Quest设备内置的浏览器访问此工具。</p>
        <p>非常不幸，<b>你无法在想要注入模组的设备本身上使用MBF。</b></p>
    </>,

    onlyWorkWithAnotherQuest:<>（如果你安装了一个Chromium浏览器，那么MBF可以在Quest上使用，但这只能用来给通过USB连接的<b>另外一台Quest设备</b>注入模组。）</>,

    deviceSupportingModding:<>
        <p>如果你想要给游戏注入模组，那么你需要这些设备之一： </p>
        <ul>
          <li>（最好是这个）一台电脑或者Mac</li>
          <li>（这个也能用）一台安卓手机</li>
        </ul>
    </>,

    iosNotSupported:<>
        <h1>不支持iOS设备</h1>
        <p>MBF检测到你正在使用iOS设备。非常不幸，苹果不允许使用WebUSB，而MBF需要用这个功能与Quest设备进行交互。</p>
    </>,

    supportedBrowserHintInIOS:<>
        …… 以及下面这些浏览器：
    </>,

    browserNotSupported:<>
        <h1>浏览器不支持</h1>
        <p>看起来你正在用的浏览器不支持WebUSB，但是这个应用需要用此功能来访问你的Quest设备。</p>
    </>,
    supportedBrowserTitle:<>支持的浏览器</>,

    supportedBrowserMobile:<>
        <ul>
            <li>安卓版本的Google Chrome 122或更新版本</li>
            <li>安卓版本的Edge浏览器123或更新版本</li>
        </ul>
        <h3 className='fireFox'>不支持安卓版本的FireFox</h3>
    </>,
    supportedBrowserNotMobile:<>
        <ul>
            <li>Google Chrome 61或更新版本</li>
            <li>Opera 48或更新版本</li>
            <li>Microsoft Edge 79或更新版本</li>
        </ul>
        <h3 className='fireFox'>不支持FireFox或Safari浏览器。</h3>
        <p>（无法支持FireFox浏览器，因为Mozilla出于安全考虑不支持WebUSB特性）</p>
    </>,

    noCompatableDeviceMobile:<>
        <h4>正在使用安卓设备？</h4>
        <p>很可能你没有正确设置你的设备与Quest之间的连接方式。若要修复：</p>
        <ul>
            <li>在安卓设备顶部下滑打开通知栏。</li>
            <li>点按和USB连接有关的选项。可能叫做“正在通过 USB 为此设备充电”。</li>
            <li>将“USB受控于”选项修改至“已连接的设备”。如果“已连接的设备”已经被选中，就先改成“本机”，然后再改回来。</li>
        </ul>
        <h4>还是不行?</h4>
        <p>尝试把你的c-to-c数据线两端调换一下，让手机这头插到Quest上面。</p>
    </>,

    chooseCoreModUrl:<>
        <h1>手动覆写核心模组JSON文件</h1>
        <p>请输入一个完成的URL，内容包含你自己的核心模组JSON文件</p>
    </>,

    confirmUrl:<>确认URL</>,

    versionSupportedInstallEssentialMods: <>
        <p>为了让游戏能够使用自制歌曲，ModsBeforeFriday接下来会给游戏打补丁，并安装一些重要的模组。
            结束之后，你就可以<b>在游戏里面</b>管理你的自制歌曲。</p>
    </>,
    
    olderThanLatestModdableHint: <>你选择了一个比最新可用模组版本更旧的游戏版本。<b>应该只在有明确原因时，才这样做！</b></>,

    reverseDecision: <>（撤回决定）</>,

    quest1ModHint: (version:string)=>
        <>MBF检测到你正在使用Quest 1。为了使用最新模组，MBF会将你的游戏升级至最新可用模组的版本（{version}）。
                虽然Meta官方只为Quest 1支持到节奏光剑 v1.36.2，但是MBF可以通过补丁安装至{version}，所以依旧可以用！
            </>
    ,

    unsupportedVersion: <>此版本不支持</>,
    readThisMessage: <>在提问前，请阅读这个信息！</>,
    notSupportedModsText: (version:string, isLegacy:boolean)=>
        <>你安装了节奏光剑 v{version}但这个版本不支持{isLegacy ? "使用MBF来注入模组" : "注入模组"}！</>
    ,

    legacyUpdateRecommand:<>
        <p>你用的这个版本或许可以通过其它模组工具来注入模组。但是，<b>强烈建议</b>你把它卸载掉，然后安装最新的可用模组的版本。</p>
        <p className="warning"><em>BSMG（节奏光剑模组社区）已经不在支持</em>为1.28.0或更低的游戏版本注入模组——这是一个老旧的游戏版本，所有人都不应该用它了。务必务必<em>务必</em>升级至最新的游戏版本。</p>
    </>,

    normallyUpdateRecommand:<>
        <p>一般来说，MBF会尝试将你的游戏降级到一个支持模组的游戏版本上，但前提是你要安装最新版本的游戏。</p>
        <p>请按下面的按钮来卸载游戏，然后重新从Meta商店安装最新版本的游戏。</p>
    </>,

    awaitingPatchGeneration: <>正在等待生成补丁</>,

    mustReadMessageFull:<>你必须<b>全文阅读</b>以下内容。</>,

    noDiffMessageBody: (version:string)=>
        <>
            <p>你安装了节奏光剑，版本号是 v{version}暂时不支持模组。</p>
            <p>MBF被设计为可以降级游戏至一个支持模组的版本， <b>但是现在还没有生成必要的补丁文件，</b>因为节奏光剑官方游戏刚刚更新。</p>
            <p>生成补丁这件工作需要手动操作，<b>MBF的作者在空闲时很快就会进行</b>，这会持续大概半小时到一天左右的时间。</p>
            <p><b>请耐心等待。</b>你可以刷新这个页面，然后重新连接Quest设备，来看看补丁是不是已经生成好了。</p>
        </>
    ,

    incompatableModLoader: (modLoader:string)=>
        <>
            <h1>模组加载器不兼容</h1>
            <p>你的游戏已经带有了{modLoader === 'QuestLoader' ? "QuestLoader" : "某个未知的"}模组加载器，MBF不支持这个加载器。</p>
            <p>你必须卸载游戏，然后重新安装最新的原版游戏，这样就可以使用Scotland2模组加载器。</p>
            <p>不必担心！你的自制歌曲不会丢失。</p>
        </>
    ,

    incompatableVersionPatched: (version:string)=><>
            <h1>已补丁的版本不支持模组</h1>

            <p>你的游戏已经安装了一个模组加载器，但这个版本（{version}）没有支持的模组!</p>
            <p>如果要修复，卸载节奏光剑并重新安装最新版本。然后MBF会自动降级游戏至最新可安装模组的版本。</p>
        </>
    ,

    obbNotPresent:<>
        <h1>没有找到OBB</h1>
        <p>MBF检测到OOB文件（包含了节奏光剑启动所需的游戏资源）没有被正确安装。</p>
        <p>这说明你的游戏是损坏的。你需要点击下面的按钮卸载游戏，然后在Meta商店重新安装最新版本。</p>
    </>,

    coreModDisabled:<>核心模组已禁用。</>,

    problemFound:<>在安装过程中发现了问题：</>,
    problemCanFix:<>可以点击下面的按钮来轻松修复。</>,
    modloaderNotFound:<>没有找到模组加载器</>,
    modloaderNeedUpdate:<>模组加载器有更新</>,
    coremodsMissing:<>有些核心模组没有安装</>,
    coreModsNeedUpdate:<>需要安装核心模组的更新。</>,
    fixIssue:<>修复问题</>,

    changeManifestXmlHint:<>
        <h2>修改清单XML</h2>
        <p>仅用于开发，这个菜单能让你手动编辑APK的AndroidManifest.xml文件。</p>
        <p>务必小心，错误的编辑会导致APK安装异常。</p>
    </>,
    
    // permMenuPermissions:<>Permissions</>,       // 不翻译
    // permMenuFeatures:<>Features</>,             // 不翻译
}