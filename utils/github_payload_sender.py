import argparse
import hashlib
import hmac
import json
import sys
from pathlib import Path

import yaml
from requests import ConnectionError, post

DEFAULT_URL = 'http://127.0.0.1:1234/api/webhooks/github'


class Event:
    def __init__(self, type_: str, json_data: str):
        self.type_ = type_
        self.json_data = json_data
        self.signature = ''

    def sign(self, secret: str):
        self.signature = hmac.new(secret.encode(), self.json_data.encode(), hashlib.sha256).hexdigest()

    def send(self, url):
        headers = {
            'Content-Type': 'application/json',
            'X-GitHub-Event': self.type_,
            'X-Hub-Signature-256': f'sha256={self.signature}',
        }

        try:
            post(url, data=self.json_data, headers=headers)
        except ConnectionError:
            print(f'Unable to send event to {url}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()

    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument('--github-secret', metavar='secret', help='GitHub secret used to sign the payload data')
    group.add_argument('--config',
                       metavar='file',
                       help='prololo config file (yaml) to retrieve the GitHub secret from',
                       type=Path)

    parser.add_argument('--url', metavar='url', help='URL address to send webhooks to', default=DEFAULT_URL)

    parser.add_argument('payload_files', metavar='payload-file', help='payload to send', nargs='+', type=Path)

    args = parser.parse_args()

    if args.github_secret:
        secret = args.github_secret
    elif args.config and args.config.exists():
        with open(args.config, 'r') as config_file:
            config = yaml.full_load(config_file)
            secret = config.get('github_secret', None)
        if not secret:
            sys.exit(f'{args.config} does not seems to contain a valid github_secret!')

    for file_ in args.payload_files:
        if file_.exists():
            with open(file_, 'r') as json_file:
                json_data = json.dumps(json.load(json_file), indent=None, separators=(',', ':'))
                event = Event(file_.stem.split('-')[0], json_data)
                event.sign(secret)
                event.send(args.url)
