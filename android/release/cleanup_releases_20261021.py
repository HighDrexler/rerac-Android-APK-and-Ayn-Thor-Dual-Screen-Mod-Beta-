"""One-time owner-requested cleanup; exact release IDs only, never source tags."""
import json, subprocess, sys, urllib.parse
from pathlib import Path

ROOT = Path('work/release-cleanup')
PLAN = Path('android/release/release-cleanup-20261021.json')
EXPECTED = {
  419105836: 'rerac-android-0.3.6-dev.6',
  419063719: 'rerac-android-0.3.6-dev.5',
  419030255: 'rerac-android-0.3.6-dev.4',
  419996071: 'beta17-signing-input',
  418989697: 'rerac-android-0.3.6-dev.3',
  418966977: 'rerac-android-0.3.6-dev.2',
  418849207: 'rerac-android-0.3.6-dev.1',
  418743677: 'rerac-android-0.3.5-dev.4',
  418690071: 'rerac-android-0.3.5-dev.3',
  418125056: 'rerac-android-0.3.5-dev.2',
  418022925: 'rerac-android-0.3.5-dev.1',
  417813397: 'launcher-v1.1.0-beta.3',
  417720128: 'launcher-v1.1.0-beta-test',
}
KEEP = {420001807: 'rerac-android-0.3.6-dev.8',419167879:'rerac-android-0.3.6-dev.7',416785922:'v1.0'}
REPO = 'HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-'

def api(path, method='GET', body=None):
    command=['gh','api','--method',method,'repos/'+REPO+'/'+path]
    if body is not None: command+=['--input','-']
    result=subprocess.run(command,input=json.dumps(body) if body is not None else None,capture_output=True,text=True,check=True)
    return json.loads(result.stdout) if result.stdout.strip() else None

def feeds():
    return {branch+'/'+channel:api('contents/'+channel+'.json?ref='+branch)['sha']
            for branch in ('update-channel2','update-channel3') for channel in ('stable','development')}

def tags():
    return {tag:api('git/ref/tags/'+urllib.parse.quote(tag,safe=''))['object'] for tag in list(EXPECTED.values())+list(KEEP.values())}

def validate_plan():
    p=json.loads(PLAN.read_text())
    assert p['schema']==1 and p['repository']==REPO and p['preserve_git_tags'] is True
    assert {x['id']:x['tag'] for x in p['remove']}==EXPECTED
    assert {x['id']:x['tag'] for x in p['keep']}==KEEP
    assert not (EXPECTED.keys() & KEEP.keys())

def main():
    validate_plan(); ROOT.mkdir(parents=True,exist_ok=True)
    mode=sys.argv[1]
    if mode=='snapshot':
        releases=api('releases?per_page=100')
        live={r['id']:r for r in releases}
        assert KEEP.keys() <= live.keys()
        for rid,tag in {**EXPECTED,**KEEP}.items():
            if rid in live: assert live[rid]['tag_name']==tag
        snapshot=dict(releases=releases,tags=tags(),feeds=feeds())
        (ROOT/'before.json').write_text(json.dumps(snapshot,indent=2)+'\n')
        print('Saved public release metadata and tag/feed references before cleanup.')
        return
    assert mode=='apply'
    before=json.loads((ROOT/'before.json').read_text())
    assert feeds()==before['feeds']
    for rid,tag in EXPECTED.items():
        live={r['id']:r for r in api('releases?per_page=100')}
        if rid not in live: continue
        release=live[rid]
        assert release['tag_name']==tag and release['prerelease'] is True
        assert rid not in KEEP
        api('releases/'+str(rid),'DELETE')
        print('Removed superseded release:',tag)
    current=api('releases/420001807')
    api('releases/420001807','PATCH',{'name':'ReRAC Android 0.3.6 beta.17 - current test build'})
    legacy=api('releases/419167879')
    notice='LEGACY UPDATER DOWNLOAD: retained for launchers signed with the previous key. New testing should use beta.17, which establishes the new signing/update baseline.\n\n'
    api('releases/419167879','PATCH',{'name':'Legacy updater download - beta.16','body':notice+legacy['body'] if not legacy['body'].startswith('LEGACY UPDATER DOWNLOAD:') else legacy['body']})
    after=api('releases?per_page=100')
    actual={r['id']:r['tag_name'] for r in after}
    assert KEEP.keys() <= actual.keys() and not EXPECTED.keys() & actual.keys()
    assert tags()==before['tags'], 'A source tag changed'
    assert feeds()==before['feeds'], 'An update feed changed'
    result=dict(status='passed',remaining=[dict(id=r['id'],tag=r['tag_name'],name=r['name']) for r in after],
                removed_ids=list(EXPECTED),source_tags_unchanged=True,update_feeds_unchanged=True)
    (ROOT/'after.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))

if __name__=='__main__':main()
