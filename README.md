# ModsBeforeFriday

ModsBeforeFriday is a modding tool for Beat Saber on Quest that works entirely within the browser, using WebUSB to interact with the quest. The aim is to make installing mods as easy as possible, with no need to download special tools or hunt around for core mods.

## Project Structure

`./mbf-agent` contains the agent, which is an executable written in Rust that is executed by the frontend via ADB. This agent does pretty much all the work, including installing mods and patching the game.
`./mbf-site` contains the frontend, which communicates with the agent via JSON. (Written in typescript with React).

## Compilation Instructions
... coming soon

### TODO List
- Make the "fix issues" button actually do something.
- Add actions and build instructions
- Do some beta testing
- Deploy