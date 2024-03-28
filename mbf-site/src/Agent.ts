import { AdbSync, AdbSyncWriteOptions, Adb, encodeUtf8 } from '@yume-chan/adb';
import { ConsumableReadableStream, Consumable, DecodeUtf8Stream, ConcatStringStream } from '@yume-chan/stream-extra';
import { Request } from "./Messages";

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
  try {
    console.log("Removing existing agent");
    await adb.subprocess.spawnAndWait("rm " + AgentPath)
    console.log("Writing new agent")
    await saveAgent(sync);
    await adb.subprocess.spawnAndWait("chmod +x " + AgentPath);
  } finally {
    sync.dispose();
  }

  console.log("Agent is ready");
}
  
async function downloadAgent(): Promise<Uint8Array> {
    const resp: Response = await fetch("./mbf-agent/mbf-agent");
    if(resp.body === null) {
        throw new Error("Agent response had no body")
    }

    return new Uint8Array(await resp.arrayBuffer());
}


async function sendRequest(adb: Adb, request: Request): Promise<Response> {
  let command_buffer = encodeUtf8(JSON.stringify(request) + "\n");

  let agentProcess = await adb.subprocess.spawn(AgentPath);

  const stdin = agentProcess.stdin.getWriter();
  try {
    stdin.write(new Consumable(command_buffer));
  } finally {
    stdin.releaseLock();
  }

  console.log("Waiting for exit");
  const code = await agentProcess.exit;
  if(code === 0) {
    console.log("Parsing output");
    return JSON.parse(await agentProcess.stdout
      .pipeThrough(new DecodeUtf8Stream())
      .pipeThrough(new ConcatStringStream()));
  } else  {
    console.log("Parsing error output")
    throw new Error("Received error response from agent: " + 
    await agentProcess.stderr
      .pipeThrough(new DecodeUtf8Stream())
      .pipeThrough(new ConcatStringStream()))
  }
}

export {
  prepareAgent,
  sendRequest as runCommand
};