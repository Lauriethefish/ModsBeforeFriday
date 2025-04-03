export const gameId = new URLSearchParams(window.location.search).get("game_id") || "com.beatgames.beatsaber"
export const ignorePackageId = new URLSearchParams(window.location.search).get("ignore_package_id") == "true"
