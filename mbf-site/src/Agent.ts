import { AdbSync, AdbSyncWriteOptions, Adb, encodeUtf8 } from '@yume-chan/adb';
import { ConsumableReadableStream, Consumable, DecodeUtf8Stream, ConcatStringStream } from '@yume-chan/stream-extra';
import { Request, Response, LogMsg } from "./Messages";

const AgentPath: string = "/data/local/tmp/mbf-agent";

async function saveAgent(sync: AdbSync) {
  console.log("Downloading agent");
  const agent: Uint8Array = await downloadAgent();

  // TODO: properly use readable streams
  const file: ConsumableReadableStream<Uint8Array> = new ConsumableReadableStream({
    start(controller) {
      controller.enqueue(agent);
      controller.close();
    },
  });

  const options: AdbSyncWriteOptions = {
    filename: AgentPath,
    file
  };

  await sync.write(options);
}

async function prepareAgent(adb: Adb) {
  const sync = await adb.sync();
  console.group("Pushing agent");
  try {
    console.log("Removing existing agent");
    await adb.subprocess.spawnAndWait("rm " + AgentPath)
    console.log("Writing new agent")
    await saveAgent(sync);
    await adb.subprocess.spawnAndWait("chmod +x " + AgentPath);

    console.log("Agent is ready");
  } finally {
    sync.dispose();
    console.groupEnd();
  }
}
  
async function downloadAgent(): Promise<Uint8Array> {
    const resp = await fetch("./mbf-agent/mbf-agent");
    if(resp.body === null) {
        throw new Error("Agent response had no body")
    }

    return new Uint8Array(await resp.arrayBuffer());
}

function logFromAgent(log: LogMsg) {
  switch(log.level) {
    case 'Error':
      console.error(log.message);
      break;
    case 'Warn':
      console.warn(log.message);
      break;
    case 'Debug':
      console.debug(log.message);
      break;
    case 'Info':
      console.info(log.message);
      break;
    case 'Trace':
      console.trace(log.message);
  }
}

async function sendRequest(adb: Adb, request: Request, eventSink: ((event: LogMsg) => void) | null = null): Promise<Response> {
  let command_buffer = encodeUtf8(JSON.stringify(request) + "\n");

  let agentProcess = await adb.subprocess.shell(AgentPath);

  const stdin = agentProcess.stdin.getWriter();
  try {
    stdin.write(new Consumable(command_buffer));
  } finally {
    stdin.releaseLock();
  }

  let exited = false;
  agentProcess.exit.then(() => exited = true);

  const reader = agentProcess.stdout
    // TODO: Not totally sure if this will handle non-ASCII correctly.
    // Doesn't seem to consider that a chunk might not be valid UTF-8 on its own
    .pipeThrough(new DecodeUtf8Stream())
    .getReader();
  
  console.group("Agent Request");
  let buffer = "";
  let response: Response | null = null;
  while(!exited) {
    const result = await reader.read();
    const receivedStr = result.value;
    if(receivedStr === undefined) {
      continue;
    }

    // TODO: This is fairly inefficient in terms of memory usage
    // (although we aren't receiving a huge amount of data so this might be OK)
    buffer += receivedStr;
    const messages = buffer.split("\n");
    buffer = messages[messages.length - 1];

    for(let i = 0; i < messages.length - 1; i++) {
      // Parse each newline separated message as a Response
      let msg_obj: Response;
      try {
        msg_obj = JSON.parse(messages[i]) as Response;
      } catch(e) {
        throw new Error("Agent message " + messages[i] + " was not valid JSON");
      }
      if(msg_obj.type === "LogMsg") {
        const log_obj = msg_obj as LogMsg;
        logFromAgent(log_obj);
        if(eventSink != null) {
          eventSink(log_obj);
        }

        // Errors need to be thrown later in the function
        if(msg_obj.level === 'Error') {
          response = msg_obj;
        }
      } else  {
        // The final message is the only one that isn't of type `log`.
        // This contains the actual response data
        response = msg_obj;
      }
    }
  }
  console.groupEnd();

  if((await agentProcess.exit) === 0) {
    if(response === null) {
      throw new Error("Received error response from agent");
    } else if(response.type === 'LogMsg') {
      const log = response as LogMsg;
      throw new Error("Received error from backend: `" + log.message + "`");
    } else  {
      return response;
    }
  } else  {
    // If the agent exited with a non-zero code then it failed to actually write a response to stdout
    // Alternatively, the agent might be corrupt.
    throw new Error("Failed to invoke agent: is the executable corrupt?" + 
      await agentProcess.stderr
        .pipeThrough(new DecodeUtf8Stream())
        .pipeThrough(new ConcatStringStream()))
  }
}

export {
  prepareAgent,
  sendRequest as runCommand
};