#!/usr/bin/env python3
"""Execute the shipped webhook step with real jq and a capturing curl fixture."""
import json, os, pathlib, subprocess, tempfile
root=pathlib.Path(__file__).resolve().parents[1]
text=(root/'templates/github/buzz-notify.yml').read_text()
script='\n'.join(line[10:] for line in text.split('        run: |\n',1)[1].splitlines())+'\n'
with tempfile.TemporaryDirectory(prefix='buzz-kit-events-') as tmp:
    folder=pathlib.Path(tmp)
    curl=folder/'curl'
    curl.write_text('#!/usr/bin/env python3\nimport sys,json,os,pathlib\np=json.load(sys.stdin)\nassert "--fail-with-body" in sys.argv\npathlib.Path(os.environ["CAPTURE"]).write_text(json.dumps(p))\n')
    curl.chmod(0o755)
    title='untrusted $(touch injected) `touch injected` "quote"\nnext line'
    env=dict(os.environ,PATH=tmp+':'+os.environ['PATH'],CAPTURE=str(folder/'capture'),BUZZ_WEBHOOK_URL='https://example.com/hooks/fixture',BUZZ_WEBHOOK_SECRET='synthetic',REPO='example/project',TITLE=title,URL='https://example.com/pr/1',ACTOR='fixture',PR_MERGED='false',CI_CONCLUSION='')
    cases=[('pull_request','opened','false','','PR opened'),('pull_request','reopened','false','','PR reopened'),('pull_request','ready_for_review','false','','PR ready_for_review'),('pull_request','closed','true','','PR merged'),('pull_request','closed','false','','PR closed'),('issues','opened','false','','issue opened'),('issues','closed','false','','issue closed'),('release','published','false','','release published'),('workflow_run','completed','false','failure','CI failed'),('workflow_run','completed','false','success',None),('pull_request','synchronize','false','',None)]
    for event,action,merged,conclusion,expected in cases:
        env.update(EVENT_NAME=event,EVENT_ACTION=action,PR_MERGED=merged,CI_CONCLUSION=conclusion)
        capture=folder/'capture';capture.unlink(missing_ok=True)
        subprocess.run(['bash','-c',script],env=env,cwd=folder,check=True)
        if expected is None: assert not capture.exists()
        else:
            data=json.loads(capture.read_text());assert set(data)=={'repo','event','title','url','actor'};assert data['event']==expected;assert data['title']==title
        assert not (folder/'injected').exists()
    env.update(EVENT_NAME='pull_request',EVENT_ACTION='opened',BUZZ_WEBHOOK_SECRET='')
    capture.unlink(missing_ok=True);subprocess.run(['bash','-c',script],env=env,cwd=folder,check=True);assert not capture.exists()
print('PASS: event matrix, flat JSON, hostile titles, successful-CI skip and missing fork secret')
