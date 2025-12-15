import { Eng } from "./en";

export const SimplifiedChinese = {
    __proto__: Eng, // use english text for missing items

    sourceCode: <>源代码</>,
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

    creditsOkBtnText:<>OK</>,

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
}