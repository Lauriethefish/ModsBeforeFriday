import { AdbSync, AdbSyncWriteOptions, Adb, encodeUtf8 } from '@yume-chan/adb';
import { PromiseResolver } from '@yume-chan/async';
import { Consumable, TextDecoderStream, MaybeConsumable, ReadableStream, WritableStream } from '@yume-chan/stream-extra';
import { Request, Response, LogMsg, ModStatus, Mods, FixedPlayerData, ImportResult, DowngradedManifest, Patched, ModSyncResult } from "./Messages";
import { AGENT_SHA1 } from './agent_manifest';
import { toast } from 'react-toastify';
import { Log } from './Logging';

const AgentPath: string = "/data/local/tmp/mbf-agent";
const UploadsPath: string = "/data/local/tmp/mbf/uploads/";

// Converts the provided byte array into a ReadableStream that can be fed into ADB.
function readableStreamFromByteArray(array: Uint8Array): ReadableStream<Uint8Array> {
  return new ReadableStream({
    start(controller) {
      controller.enqueue(array);
      controller.close();
    },
  });
}

export async function prepareAgent(adb: Adb) {
  Log.info("Preparing agent: used to communicate with your Quest.");

  Log.debug("Latest agent SHA1 " + AGENT_SHA1);

  const existingSha1 = (await adb.subprocess.spawnAndWait(`sha1sum ${AgentPath} | cut -f 1 -d " "`)).stdout
    .trim()
    .toUpperCase();
  Log.debug("Existing agent SHA1: " + existingSha1);
  const existingUpToDate = AGENT_SHA1 == existingSha1.trim().toUpperCase();
  if(existingUpToDate) {
    Log.info("Agent is up to date");
  } else  {
    await overwriteAgent(adb);
  }
}

export async function overwriteAgent(adb: Adb) {
  const sync = await adb.sync();
  console.group("Downloading and overwriting agent on Quest");
  try {
    Log.debug("Removing existing agent");
    await adb.subprocess.spawnAndWait("rm " + AgentPath)
    Log.debug("Downloading agent, this might take a minute")
    await saveAgent(sync);
    Log.debug("Making agent executable");
    await adb.subprocess.spawnAndWait("chmod +x " + AgentPath);

    Log.info("Agent is ready");
  } finally {
    sync.dispose();
    console.groupEnd();
  }
}

async function saveAgent(sync: AdbSync) {
  // Timeout, in seconds, before the app will treat the agent upload as failed and terminate the connection.
  const AGENT_UPLOAD_TIMEOUT: number = 30;

  const agent: Uint8Array = await downloadAgent();
  const file: ReadableStream<MaybeConsumable<Uint8Array>> = readableStreamFromByteArray(agent);

  const options: AdbSyncWriteOptions = {
    filename: AgentPath,
    file
  };

  Log.info("Writing agent to quest!");
  const timeoutPromise = new Promise((_, reject) => {
    setTimeout(() => reject(new Error(`Did not finish pushing agent after ${AGENT_UPLOAD_TIMEOUT} seconds.\n`
        + `In practice, pushing the agent takes less than a second, so this is a bug. Please report this issue including information about `
        + `which web browser you are using.`
    )), AGENT_UPLOAD_TIMEOUT * 1000);
  });

  await Promise.race([timeoutPromise, sync.write(options)])
}
  
async function downloadAgent(): Promise<Uint8Array> {
  const MAX_ATTEMPTS: number = 3;
  const PROGRESS_UPDATE_INTERVAL = 1000; // Time between download progress updates, in milliseconds

  let ok = false;
  let attempt = 1;
  do {
    try {
      // Use XMLHttpRequest adapted to work with promises/async to fetch the agent
      // Previously this used the fetch API, and there was some suggestion that various issues regarding the download
      // "hanging" before data was received were caused by fetch.
      // So, to see if it fixes the problem, we have changed to XMLHttpRequest.
      const xhr = new XMLHttpRequest();
      await new Promise((resolve, reject) => {
        xhr.open('GET', "mbf-agent", true);
        xhr.responseType = "arraybuffer";

        xhr.onload = function() {
          if(xhr.status >= 200 && xhr.status < 300) {
            resolve(xhr.response);
          } else  {
            reject(xhr.status)
          }
        };
        xhr.onerror = function() {
          reject(xhr.status)
        }

        let lastReadTime = new Date().getTime();
        xhr.onprogress = function(event) {
          if(!event.lengthComputable) {
            return;
          }

          // Do not spam with progress updates: only every second or so
          const timeNow = new Date().getTime();
          if(timeNow - lastReadTime > PROGRESS_UPDATE_INTERVAL) {
            // Calculate the percentage of the download that has completed
            const percentComplete = (event.loaded / event.total) * 100.0;
            lastReadTime = timeNow;
            Log.info(`Download ${Math.round(percentComplete * 10) / 10}% complete`);
          }
        }

        xhr.send();
      })

      return new Uint8Array(xhr.response);
    } catch(e) {
      Log.error("Failed to fetch agent, status " + e, e);
    }

    attempt++;
    if(attempt <= MAX_ATTEMPTS) {
      Log.info(`Failed to download agent, trying again... (attempt ${attempt}/${MAX_ATTEMPTS})`);
    }
  } while(!ok && attempt <= MAX_ATTEMPTS);

  throw new Error("Failed to fetch agent after multiple attempts.\nDid you lose internet connection just after you loaded the site?\n\nIf not, then please report this issue, including a screenshot of the browser console window!");
}

/**
 * Creates a WritableStream that can be used to log messages from the agent.
 * @returns
 */
function createLoggingWritableStream<ChunkType>(chunkCallback?: (chunks: ChunkType[], closed: boolean, error: unknown) => any): { promise: Promise<ChunkType[]>, stream: WritableStream<ChunkType> } {
  let streamResolver = new PromiseResolver<ChunkType[]>();
  let chunks: ChunkType[] = [];
  let promiseResolved = false;

  setTimeout(async () => {
    await streamResolver.promise
      .then(() => chunkCallback?.(chunks, true, undefined))
      .catch((error) => chunkCallback?.(chunks, true, error))
      .finally();

    promiseResolved = true;
  });

  // Create a WritableStream that will log messages from the agent
  const stream = new WritableStream<ChunkType>({
    write(chunk) {
      chunks.push(chunk);
      chunkCallback?.(chunks, false, undefined);
    },
    close() {
      chunkCallback?.(chunks, true, undefined);
      !promiseResolved && streamResolver.resolve(chunks);
    },
    abort(error) {
      chunkCallback?.(chunks, true, error);
      !promiseResolved && streamResolver.reject({chunks, error});
    },
  });

  return {
    promise: streamResolver.promise,
    stream
  }
}

async function sendRequest(adb: Adb, request: Request): Promise<Response> {
  let command_buffer = encodeUtf8(JSON.stringify(request) + "\n");
  let agentProcess = await adb.subprocess.spawn(AgentPath);
  let response = null as (Response | null); // Typescript is weird...
  const stdin = agentProcess.stdin.getWriter();

  try {
    stdin.write(new Consumable(command_buffer));
  } finally {
    stdin.releaseLock();
  }

  console.group("Agent Request");
  console.log(request);

  console.group("Messages");

  // Create a WritableStream that will log messages from the agent stdout.
  // The stream will run the callback function when it receives a chunk of data.
  const {stream: outputCaptureStream, promise: outputCapturePromise} = createLoggingWritableStream((chunks: string[], closed, error) => {
    // Combine all the chunks into a single string, then split it by newline.
    // Splice also clears the chunks array.
    const messages: string[] = chunks.splice(0, chunks.length).join("").split("\n");

    // If not closed, the last message is incomplete and should be put back into the chunks array
    if (!closed) {
      const lastMessage = messages.pop()!;
      chunks.unshift(lastMessage);
    }

    // Parse each message
    for (const message of messages.filter(m => m.trim())) {
      let msg_obj: Response;

      // Try to parse the message as JSON
      try {
        msg_obj = JSON.parse(message) as Response;
      } catch(e) {
        // If the message is not valid JSON, log it and throw an error
        console.log(message);
        throw new Error("Agent message " + message + " was not valid JSON");
      }

      // If the message is a log message, emit it to the global log store
      if(msg_obj.type === "LogMsg") {
        const log_obj = msg_obj as LogMsg;
        Log.emitEvent(log_obj);

        // Errors need to be thrown later in the function
        if(msg_obj.level === 'Error') {
          response = msg_obj;
        }

        continue;
      }

      // The final message is the only one that isn't of type `log`.
      // This contains the actual response data
      console.log(msg_obj);
      response = msg_obj;
    }
  });
  outputCapturePromise.finally(console.groupEnd);

  // Create a WritableStream that will log messages from the agent stderr.
  const {stream: errorCaptureStream, promise: errorCapturePromise} = createLoggingWritableStream<string>();

  // Pipe the agent stdout and stderr to the logging streams
  agentProcess.stderr.pipeThrough(new TextDecoderStream()).pipeTo(errorCaptureStream);
  agentProcess.stdout.pipeThrough(new TextDecoderStream()).pipeTo(outputCaptureStream);

  // Wait for everything to finish
  let [exitCode, outputChunks, errorChunks] = await Promise.all([
    agentProcess.exit,
    outputCapturePromise,
    errorCapturePromise
  ]);

  console.log(`Exited: ${exitCode}`);
  console.groupEnd();

  if(exitCode === 0) {
    if(response !== null && (response as LogMsg).level === 'Error') {
      throw new Error("Agent responded with an error", { cause: response });
    } else {
      return response as Response;
    }
  } else  {
    // If the agent exited with a non-zero code then it failed to actually write a response to stdout:
    // Since the agent in its current form catches all panics and other errors it should always return exit code 0
    // Hence, the agent is most likely corrupt or not executable for some other reason.
    // We will delete the agent before we quit so it is redownloaded next time MBF is restarted.
    await adb.subprocess.spawnAndWait("rm " + AgentPath)

    throw new Error("Failed to invoke agent: is the executable corrupt or permissions not properly set?\nThe agent has been deleted automatically: refresh the page and the agent will be redownloaded, hopefully fixing the problem: \n\n" +
      errorChunks.join(""))
  }
}

let CORE_MOD_OVERRIDE_URL: string | null = null;
export function setCoreModOverrideUrl(core_mod_override_url: string | null) {
  CORE_MOD_OVERRIDE_URL = core_mod_override_url;
}

// Gets the status of mods from the quest, i.e. whether the app is patched, and what mods are currently installed.
export async function loadModStatus(device: Adb): Promise<ModStatus> {
  await prepareAgent(device);

  return await sendRequest(device, {
      type: 'GetModStatus',
      override_core_mod_url: CORE_MOD_OVERRIDE_URL,
  }) as ModStatus;
}

// Tells the backend to attempt to uninstall/install the given mods, depending on the new install status provided in `changesRequested`.
export async function setModStatuses(device: Adb,
  changesRequested: { [id: string]: boolean }): Promise<ModSyncResult> {
  let response = await sendRequest(device, {
      type: 'SetModsEnabled',
      statuses: changesRequested
  });

  return response as ModSyncResult;
}

// Gets the AndroidManifest.xml file for the given Beat Saber APK version, converted from AXML to XML.
export async function getDowngradedManifest(device: Adb, gameVersion: string): Promise<string> {
  let response = await sendRequest(device, {
    type: 'GetDowngradedManifest',
    version: gameVersion
  });

  return (response as DowngradedManifest).manifest_xml;
}

export async function importFile(device: Adb,
    file: File): Promise<ImportResult> {
  const sync = await device.sync();
  const tempPath = UploadsPath + file.name;
  try {
    
    Log.debug("Uploading to " + tempPath);

    await sync.write({
      filename: tempPath,
      file: readableStreamFromByteArray(new Uint8Array(await file.arrayBuffer()))
    })

    const response = await sendRequest(device, {
      'type': 'Import',
      from_path: tempPath
    });

    return response as ImportResult;
  } finally {
    sync.dispose();
  }
}

export async function importUrl(device: Adb,
url: string) {
  const response = await sendRequest(device, {
    type: 'ImportUrl',
    from_url: url
  });

  return response as ImportResult;
}

export async function removeMod(device: Adb,
  mod_id: string) {
  let response = await sendRequest(device, {
      type: 'RemoveMod',
      id: mod_id
  });

  return (response as Mods).installed_mods;
}

// Instructs the agent to patch the app, adding the modloader and installing the core mods.
// Updates the ModStatus `beforePatch` to reflect the state of the installation after patching.
// (will not patch if the APK is already modded - will just extract the modloader and install core mods.)
export async function patchApp(device: Adb,
  beforePatch: ModStatus,
  downgradeToVersion: string | null,
  manifestMod: string,
  remodding: boolean,
  allow_no_core_mods: boolean,
  device_pre_v51: boolean,
  splashScreen: File | null): Promise<ModStatus> {
  Log.debug("Patching with manifest: " + manifestMod);

  let splashPath: string | null = null;
  if(splashScreen !== null) {
    const sync = await device.sync();
    splashPath = UploadsPath + splashScreen.name;
    try {
      Log.debug(`Pushing splash to ${splashPath}`)
      await sync.write({
        filename: splashPath,
        file: readableStreamFromByteArray(new Uint8Array(await splashScreen.arrayBuffer()))
      })
    } finally {
      await sync.dispose();
    }
  }

  let response = await sendRequest(device, {
      type: 'Patch',
      downgrade_to: downgradeToVersion,
      manifest_mod: manifestMod,
      allow_no_core_mods: allow_no_core_mods,
      override_core_mod_url: CORE_MOD_OVERRIDE_URL,
      device_pre_v51: device_pre_v51,
      remodding,
      vr_splash_path: splashPath
  }) as Patched;

  if(response.did_remove_dlc) {
    toast.warning("MBF (temporarily) deleted installed DLC while downgrading your game. To get them back, FIRST restart your headset THEN download the DLC in-game.",
        { autoClose: false })
  }

  // Return the new mod status assumed after patching
  // (patching should fail if any of this is not the case)
  return {
      'type': 'ModStatus',
      app_info: {
          loader_installed: 'Scotland2',
          version: downgradeToVersion ?? beforePatch.app_info!.version,
          manifest_xml: manifestMod,
          obb_present: beforePatch.app_info!.obb_present
      },
      core_mods: {
          core_mod_install_status: "Ready",
          supported_versions: beforePatch.core_mods!.supported_versions,
          downgrade_versions: [],
          is_awaiting_diff: false
      },
      modloader_install_status: "Ready",
      installed_mods: response.installed_mods
  };
}

// Instructs the agent to download and install any missing/outdated core mods, as well as push the modloader to the required location.
// Should fix many common issues with an install.
export async function quickFix(device: Adb,
  beforeFix: ModStatus,
  wipe_existing_mods: boolean): Promise<ModStatus> {
  let response = await sendRequest(device, {
      type: 'QuickFix',
      override_core_mod_url: CORE_MOD_OVERRIDE_URL,
      wipe_existing_mods
  });

  // Update the mod status to reflect the fixed installation
  return {
      'type': 'ModStatus',
      app_info: beforeFix.app_info,
      core_mods: {
          core_mod_install_status: "Ready",
          supported_versions: beforeFix.core_mods!.supported_versions,
          downgrade_versions: beforeFix.core_mods!.downgrade_versions,
          is_awaiting_diff: beforeFix.core_mods!.is_awaiting_diff
      },
      installed_mods: (response as Mods).installed_mods,
      modloader_install_status: "Ready"
  }
}

// Attempts to fix the black screen issue on Quest 3.
export async function fixPlayerData(device: Adb): Promise<boolean> {
  let response = await sendRequest(device, { type: 'FixPlayerData' });

  return (response as FixedPlayerData).existed
}
