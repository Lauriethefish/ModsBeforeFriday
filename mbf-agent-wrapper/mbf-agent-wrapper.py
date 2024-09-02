#!/bin/python3

import argparse
import hashlib
import json
import os
import selectors
import shlex
import subprocess
import sys
import requests
import xml.etree.ElementTree as ET

TRACE=4
DEBUG=3
INFO=2
WARN=1
ERROR=0

LEVEL_CONVERTER = {
        'trace': TRACE,
        'debug': DEBUG,
        'info': INFO,
        'warn': WARN,
        'error': ERROR}

class Wrapper():
    def __init__(self):
        self.initialize_parsers()

        try:
            self.args = self.parser.parse_args()
        except argparse.ArgumentError:
            self.parser.print_help()
            exit(2)

        self.log_level = INFO + self.args.verbosity - self.args.quiteness

        self.verify_agent()

    def verify_agent(self):
        local_hasher = hashlib.sha1()

        with open('../mbf-site/public/mbf-agent', 'rb') as file:
            while True:
                data = file.read(65536)

                if not data:
                    break

                local_hasher.update(data)
        
        local_hash = local_hasher.hexdigest()
        remote_hash = subprocess.run(['adb', 'shell', 'sha1sum', '/data/local/tmp/mbf-agent', '|', 'cut', '-f', '1', '-d', '" "'], universal_newlines=True, stdout=subprocess.PIPE).stdout[:-1];

        if local_hash != remote_hash:
            self.log('Local and remote agent differ, updating the remote agent')
            self.log(f'Local Agent Hash:  "{local_hash}"', level=DEBUG)
            self.log(f'Remote Agent Hash: "{remote_hash}"', level=DEBUG)
            subprocess.run(['adb', 'push', '../mbf-site/public/mbf-agent', '/data/local/tmp/mbf-agent'])
            subprocess.run(['adb', 'shell', 'chmod', '+x', '/data/local/tmp/mbf-agent'])

    def interactive(self, args):
        self.parser.set_interactive()
        while True:
            command = shlex.split(input("\033[32mMBF Agent Wrapper\033[0m> "))

            if command[0].lower() == 'quit' or command[0].lower() == 'exit':
                return

            try:
                self.parser.parse_args(command)
                self.args.func(self.args)
            except argparse.ArgumentError:
                self.log('Invalid Arguments', level=ERROR)
                self.parser.print_help()

    def get_mod_status(self, args):
        self.send_payload('GetModStatus', override_core_mod_url = args.override_core_mod_url)
    
    def set_mod_statuses(self, args):
        statuses = dict()

        if args.enable:
            for mod in args.enable:
                statuses[mod] = True
        if args.disable:
            for mod in args.disable:
                statuses[mod] = False
        self.send_payload('SetModsEnabled', statuses = statuses)

    def remove_mod(self, args):
        self.send_payload('RemoveMod', id = args.mod_id)

    def import_file(self, args):
        args.enable = []
        args.disable = None

        for file in args.files:
            subprocess.run(['adb', 'push', file, f'/data/local/tmp/mbf-upload/{os.path.basename(args["file"])}'])
            self.send_payload('Import', from_path=f'/data/local/tmp/mbf-upload/{os.path.basename(self.file)}')

            if 'imported_id' in self.import_result['result']:
                args.enable += [self.import_result['result']['imported_id']]
        self.set_mod_statuses(args)

    def import_url(self, args):
        args.enable = []
        args.disable = None

        for url in args.urls:
            self.send_payload('ImportUrl', from_url=url)

            if 'imported_id' in self.import_result['result']:
                args.enable += [self.import_result['result']['imported_id']]

        self.set_mod_statuses(args)

    def get_global_mods(self, args):
        if not hasattr(args, 'override_core_mod_url'):
            args.override_core_mod_url = None

        if not hasattr(args, 'game_version'):
            args.game_version = 'auto'
            self.auto_game_version(args)

        url = f'https://mods.bsquest.xyz/{args.game_version}.json'
        self.global_mods = json.loads(requests.get(url).text)

    def show_global_mods(self, args):
        if not hasattr(self, 'global_mods'):
            self.get_global_mods(args)

        self.log('\033[1mAvailable Mods\033[0m')
        for id in self.global_mods:
            latest_version = list(self.global_mods[id].keys())[-1]
            mod = self.global_mods[id][latest_version]
            self.log(f'  \033[1m{mod["name"]}\033[0m')
            self.log(f'    ID: \033[1m{id}\033[0m')
            self.log(f'    Author: \033[1m{mod["author"]}\033[0m')
            self.log(f'    Version: \033[1mv{mod["version"]}\033[0m\n')

    def import_id(self, args):
        if not hasattr(self, 'global_mods'):
            self.get_global_mods(args)

        args.enable = []
        args.disable = None

        for id in args.ids:
            if id not in self.global_mods:
                self.log(f'The mod \'{id}\' was not found in the global API! It will not be installed', level=ERROR)
                continue

            latest_version = list(self.global_mods[id].keys())[-1]
            url = self.global_mods[id][latest_version]['download']
            self.send_payload('ImportUrl', from_url=url)

            if 'imported_id' in self.import_result['result']:
                args.enable += [self.import_result['result']['imported_id']]

        self.set_mod_statuses(args)

    def auto_game_version(self, args):
        if not hasattr(self, 'mod_status'):
            self.get_mod_status(args)

        if args.game_version == 'auto':
            supported_versions = self.mod_status['core_mods']['supported_versions']

            supported_versions.sort(key = lambda version: version.replace('_', '.').split('.'))

            args.game_version = supported_versions[-1]

    def patch(self, args):
        self.auto_game_version(args)

        args.manifest_mod = None
        args.override_core_mod_url = None

        self.get_downgraded_manifest(args)

        manifest_xml = ET.fromstring(self.manifest_mod)
        external_storage_permissions = ET.Element('uses-permission', **{'ns0:name':'android.permission.MANAGE_EXTERNAL_STORAGE'})
        manifest_xml.insert(7,external_storage_permissions)

        self.parsed_manifest = ET.tostring(manifest_xml).decode('utf8')

        payload_args = {
                'downgrade_to': args.game_version,
                'manifest_mod': self.parsed_manifest,
                'remodding': args.remodding,
                'allow_no_core_mods': args.allow_no_core_mods}

        if args.no_downgrade or args.game_version == self.mod_status['app_info']['version']:
            del payload_args['downgrade_to']

        if args.override_core_mod_url:
            payload_args['override_core_mod_url'] = args.override_core_mod_url
        
        self.send_payload('Patch', **payload_args)

    def fix_player_data(self, args):
        self.send_payload('FixPlayerData')

    def get_downgraded_manifest(self, args):
        self.auto_game_version(args)

        self.send_payload('GetDowngradedManifest', version = args.game_version)

        self.manifest_mod = self.downgraded_manifest

    def quick_fix(self, args):
        self.send_payload('QuickFix',
                          override_core_mod_url=args.override_core_mod_url,
                          wipe_existing_mods=args.wipe_existing_mods)

    def send_payload(self, payload_type, **kwargs):
        payload = {'type': payload_type} | kwargs
        self.log(json.dumps(payload), level=TRACE)

        process = subprocess.Popen(['adb', 'shell', '/data/local/tmp/mbf-agent'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        process.stdin.write(json.dumps(payload).encode('utf8'))
        process.stdin.close()

        selector = selectors.DefaultSelector()
        selector.register(process.stdout, selectors.EVENT_READ)
        selector.register(process.stderr, selectors.EVENT_READ)

        combined_stdout = bytes()
        while True:
            for key, _ in selector.select():
                raw_output = key.fileobj.read1()

                if not raw_output:
                    return

                if key.fileobj is process.stdout:
                    for response in raw_output.splitlines():
                        combined_stdout += response

                        try:
                            self.parse_response(json.loads(combined_stdout.decode('utf8')))
                            combined_stdout = bytes()
                        except json.decoder.JSONDecodeError:
                            pass
                else:
                    self.log(raw_output.decode('utf8'), level=ERROR)

    def parse_response(self, response):
        response_type = response['type']
        del response['type']

        if response_type == 'LogMsg':
            self.log(response['message'], level=response['level'])
            return

        if INFO <= self.log_level:
            print()

        if response_type == 'ModStatus':
            self.mod_status = response
            
            self.log('\033[1mMod Status\033[0m')

            # App Info
            if 'app_info' in response:
                self.log('  \033[1;90mApp Info\033[0m')
                self.log(f'    Loader Installed: {response["app_info"]["loader_installed"]}')
                self.log(f'    OBB Present: {self.color_bool(response["app_info"]["obb_present"])}')
                self.log(f'    Version: {response["app_info"]["version"]}\n')
                self.log(f'    Manifest XML: {response["app_info"]["manifest_xml"]}\n', level=TRACE)

            # Installed Mods
            self.log('  \033[1;32mInstalled Mods\033[0m')
            for mod in response['installed_mods']:
                self.log_mod(mod)

            # Core Mods
            if 'core_mods' in response:
                self.log('  \033[1;92mCore Mods\033[0m', level=DEBUG)

                core_status = response['core_mods']['core_mod_install_status']
                core_status_code = '\033[32m' if core_status == 'Ready' \
                        else '\033[33m' if core_status == 'NeedUpdate' \
                        else '\033[31m'
                self.log(f'    Core Mod Install Status: {core_status_code}{core_status}', level=DEBUG)
                self.log('    Supported Versions', level=DEBUG)
                for supported_version in response['core_mods']['supported_versions']:
                    self.log(f'      {supported_version}', level=DEBUG)
                self.log('    Downgrade Versions', level=DEBUG)
                for downgrade_version in response['core_mods']['downgrade_versions']:
                    self.log(f'      {downgrade_version}', level=DEBUG)

            self.log(f'    Is Awaiting Diff? {self.color_bool(response["core_mods"]["is_awaiting_diff"], flipped=True)}', level=DEBUG)

            # OBB File Warning
            if not response['app_info']['obb_present']:
                self.log(f'    OBB file not detected! BeatSaber may not start!', level=WARN)

        elif response_type == 'Mods':
            self.mods = response

            self.log('\033[1mMods\033[0m')

            # Installed Mods
            self.log('  \033[1;32mInstalled Mods\033[0m')
            for mod in response['installed_mods']:
                self.log_mod(mod)

        elif response_type == 'ModSyncResult':
            self.mod_sync_result = response

            self.log('\033[1mMod Sync Result\033[0m')

            # Installed Mods
            self.log('  \033[1;32mInstalled Mods\033[0m')
            for mod in response['installed_mods']:
                self.log_mod(mod)

            if not response['failures']:
                return

            # Failures
            self.log(f'  \033[1;31mFailures\033[0m: {response["failures"]}', level=ERROR)

        elif response_type == 'Patched':
            self.patched = response

            self.log('\033[1mPatched\033[0m')

            # Installed Mods
            self.log('  \033[1;32mInstalled Mods\033[0m')
            for mod in response['installed_mods']:
                self.log_mod(mod)

            # DLC Removal Warning
            if response['did_remove_dlc']:
                self.log('  MBF (temporarily) deleted installed DLC while downgrading your game. To get them back, FIRST restart your headset, THEN download the DLC in-game', level=WARN)
        elif response_type == 'ImportResult':
            self.import_result = response

            self.log('\033[1mImport Result\033[0m')

            import_type = response['result']['type']

            installed_mods = None
            copied_to = None
            imported_id = None

            if import_type == 'ImportedMod':
                import_type = '\033[36mMod'
                installed_mods = response['result']['installed_mods']
                imported_id = response['result']['imported_id']
            elif import_type == 'ImportedFileCopy':
                import_type = '\033[35mFile Copy'
                copied_to = response['result']['copied_to']
                imported_id = response['result']['mod_id']
            elif import_type == 'ImportedSong':
                import_type = '\033[93mSong'
            elif import_type == 'NonQuestModDetected':
                import_type = '\033[31mNON-QUEST MOD'

            self.log(f'  \033[1;96mType\033[0m: {import_type}\033[0m')

            if installed_mods:
                self.log('  \033[1;32mInstalled Mods\033[0m')
                for mod in installed_mods:
                    self.log_mod(mod)

            if copied_to:
                self.log(f'  \033[90mCopied To\033[0m: {copied_to}', level=DEBUG)

            if imported_id:
                self.log(f'  \033[1;34mID\033[0m: {imported_id}')

        elif response_type == 'FixedPlayerData':
            self.fixed_player_data = response

            self.log('\033[1mFixed Player Data\033[0m')
            self.log(f'  \033[1;95mExisted\033[0m: {self.color_bool(response["existed"])}')
        elif response_type == 'DowngradedManifest':
            self.log('\033[1mReceived Downgraded Manifest\033[0m')
            self.downgraded_manifest = response['manifest_xml']
        else:
            self.log(f'Response type {response_type} has not been implemented!', level=ERROR)
            self.log(f'Respone was {response}', level=DEBUG)

    def color_bool(self, boolean, flipped=False):
        code = '\033[32m' if (boolean ^ flipped) else '\033[31m'
        return f'{code}{boolean}\033[0m'

    def log_mod(self, mod, indents=4):
        indent = ' '*indents
        self.log(f'{indent}\033[1m{mod["name"]}\033[0m')
        enabled = mod['is_enabled']
        if enabled:
            enabled = '\033[32mTrue\033[0m'
        else:
            enabled = '\033[31mFalse\033[0m'
        self.log(f'{indent}  Enabled: {enabled}')
        self.log(f'{indent}  Core Mod: {mod["is_core"]}', level=DEBUG)
        self.log(f'{indent}  Version: {mod["version"]}')
        self.log(f'{indent}  ID: {mod["id"]}')
        self.log(f'{indent}  Game Version: {mod["game_version"]}', level=DEBUG)
        self.log(f'{indent}  Description: {mod["description"]}\n')

    def log(self, text, level=INFO):
        if type(level) == str:
            level = LEVEL_CONVERTER[level.lower()]

        if level > self.log_level:
            return

        file = sys.stdout

        if level == TRACE:
            print('\033[90m[TRACE]\033[0m ', end='')
        elif level == DEBUG:
            print('\033[90m[\033[90mDEBUG\033[90m]\033[0m ', end='')
        elif level == INFO:
            print('\033[90m[\033[37mINFO\033[90m]\033[0m  ', end='')
        elif level == WARN:
            print('\033[90m[\033[33mWARN\033[90m]\033[0m  ', end='')
        elif level == ERROR:
            file = sys.stderr
            print('\033[90m[\033[31mERROR\033[90m]\033[0m ', end='', file = file)

        print(text, file = file)

    def initialize_parsers(self):
        parser = ArgumentParser(prog='mbf-agent-wrapper', description='Automates the generation of payloads for mbf-agent for easy cli integration', epilog='Remaining Arguments')
        self.parser = parser

        parser.add_argument('-v', '--verbose', dest='verbosity', action='count', default=0, help='Output more logging data')
        parser.add_argument('-q', '--quiet', dest='quiteness', action='count', default=0, help='Disabled all stdout outputs')

        subparser = parser.add_subparsers(dest='command')
        subparser.required = True
        self.subparser = subparser

        interactive_parser = subparser.add_parser('Interactive', help='Run in interactive mode')
        interactive_parser.set_defaults(func=self.interactive)

        get_global_mods_parser = subparser.add_parser('GetGlobalMods', help='Outputs a list of all available mods for your current version')
        get_global_mods_parser.set_defaults(func=self.show_global_mods)

        get_status_parser = subparser.add_parser('GetModStatus', help='Get the current status of mods on the headset')
        get_status_parser.add_argument('-o', '--override_core_mod_url', help='Use a custom URL for core mods')
        get_status_parser.set_defaults(func=self.get_mod_status)

        set_status_parser = subparser.add_parser('SetModStatuses', help='Mass enable/disable mods')
        self.set_status_parser = set_status_parser
        set_status_parser.add_argument('-e', '--enable', nargs='+', help='A list of mod IDs to enable')
        set_status_parser.add_argument('-d', '--disable', nargs='+', help='A list of mod IDs to disable')
        set_status_parser.set_defaults(func=self.set_mod_statuses)

        get_manifest_parser = subparser.add_parser('GetDowngradeManifest', help='Get the manifest for Beat Saber')
        self.get_manifest_parser = get_manifest_parser
        get_manifest_parser.add_argument('-g', '--game_version', default='auto', help='Which version of Beat Saber to get the manifest for')
        get_manifest_parser.set_defaults(func=self.get_downgraded_manifest)

        import_parser = subparser.add_parser('Import', help='Import a mod')
        self.import_parser = import_parser
        import_subparser = import_parser.add_subparsers()

        import_file_parser = import_subparser.add_parser('File', help='Import a mod from a file')
        self.import_file_parser = import_file_parser
        import_file_parser.add_argument('files', nargs='+', help='The local file(s) to import')
        import_file_parser.set_defaults(func=self.import_file)

        import_url_parser = import_subparser.add_parser('URL', help='Import a mod from a URL')
        self.import_url_parser = import_url_parser
        import_url_parser.add_argument('urls', nargs='+', help='The URL(s) to import')
        import_url_parser.set_defaults(func=self.import_url)

        import_id_parser = import_subparser.add_parser('ID', help='Import a mod by ID')
        self.import_id_parser = import_id_parser
        import_id_parser.add_argument('ids', nargs='+', help='The ID(s) to import')
        import_id_parser.set_defaults(func=self.import_id)

        remove_parser = subparser.add_parser('RemoveMod', help='Remove/Uninstall a mod')
        self.remove_parser = remove_parser
        remove_parser.add_argument('mod_id', help='The mod ID to remove')
        remove_parser.set_defaults(func=self.remove_mod)

        patch_parser = subparser.add_parser('Patch', help='Patch the game with a mod loader')
        self.patch_parser = patch_parser
        patch_parser.add_argument('-g', '--game_version', default='auto', help='Which version of the game to downgrade to')
        patch_parser.add_argument('-a', '--allow_no_core_mods', action='store_true', help='Allows installing versions that do not have core mods')
        patch_parser.add_argument('-o', '--override_core_mod_url', help='Use a custom URL for core mods')
        patch_parser.add_argument('-n', '--no_downgrade', action='store_true', help='Use the existing beatsaber version. Do not downgrade')
        patch_parser.add_argument('-r', '--remodding', action='store_true', help='Informs the agent that it should be remodding the game')
        patch_parser.add_argument('-m', '--manifest', help='Specifies a custom .xml file to use as the app manifest')
        patch_parser.set_defaults(func=self.patch)

        self.fix_player_data_parser = subparser.add_parser('FixPlayerData', help='Fixes permissions issues which can cause a black screen in some cases')
        self.fix_player_data_parser.set_defaults(func=self.fix_player_data)

        quick_fix_parser = subparser.add_parser('QuickFix', help='Installs any missing/outdated core mods, and pushes the modloader to the required location')
        self.quick_fix_parser = quick_fix_parser
        quick_fix_parser.add_argument('-o', '--override_core_mod_url', help='Use a custom URL for core mods')
        quick_fix_parser.add_argument('-w', '--wipe_existing_mods', action='store_true', help='Wipes all existing mods')
        quick_fix_parser.set_defaults(func=self.quick_fix)

# To ignore errors in interactive mode
class ArgumentParser(argparse.ArgumentParser):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.interactive = False

    def set_interactive(self):
        self.interactive = True

    def error(self, message):
        if not self.interactive:
            super(ArgumentParser, self).error(message)

    def exit(self, code, message):
        if not self.interactive:
            super(ArgumentParser, self).exit(code, message)

def main():
    wrapper = Wrapper()
    wrapper.args.func(wrapper.args)

if __name__ == '__main__':
    main()
