#!/usr/bin/env python3
"""Isolated host/launcher fixtures test installer behavior, not release acceptance."""
import json, os, pathlib, shutil, subprocess, tempfile
root=pathlib.Path(__file__).resolve().parents[1]
HOST='''#!/bin/sh
set -eu
printf '%s %s\\n' "${0##*/}" "$*" >> "$CASE/log"
case "$*" in
 'plugin list --json')
   if [ "${COLLISION:-}" = plugin ]; then printf '[{"id":"buzz-kit@alias"}]'; else printf '[]'; fi ;;
 'plugin marketplace list --json')
   if [ "${COLLISION:-}" = marketplace ]; then printf '[{"name":"buzz-agent-kit"}]'; else printf '[]'; fi ;;
 *) [ "${FAIL_HOST:-}" != "${0##*/}" ] || exit 9
    case "$*" in *'marketplace add'*)
      if [ "${0##*/}" = codex ]; then printf '\\n# added marketplace\\n' >> "$HOME/.codex/config.toml"; else printf '\\n' >> "$HOME/.claude/plugins/known_marketplaces.json"; fi ;;
    esac ;;
esac
'''
BOOT='''#!/bin/sh
set -eu
printf 'bootstrap\\n' >> "$CASE/log"
mkdir -p "$HOME/.local/bin"
cat > "$HOME/.local/bin/buzz-kit" <<'SH'
#!/bin/sh
printf 'doctor\\n' >> "$CASE/log"
exit "${DOCTOR_EXIT:-0}"
SH
chmod +x "$HOME/.local/bin/buzz-kit"
'''
with tempfile.TemporaryDirectory(prefix='buzz-kit-installer-') as temp:
    base=pathlib.Path(temp);kit=base/'kit';kit.mkdir();shutil.copy(root/'install.sh',kit/'install.sh');(kit/'bin').mkdir();(kit/'bin/buzz-kit').write_text(BOOT);(kit/'bin/buzz-kit').chmod(0o755)
    for relative in ['plugin.json','.claude-plugin/plugin.json','.claude-plugin/marketplace.json','crates/buzz-kit/Cargo.toml']:
        dest=kit/relative;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy(root/relative,dest)
    subprocess.run(['git','init','-q','-b','master',str(kit)],check=True)
    subprocess.run(['git','-C',str(kit),'add','.'],check=True)
    subprocess.run(['git','-C',str(kit),'-c','user.name=Fixture','-c','user.email=fixture@example.com','-c','commit.gpgsign=false','commit','-qm','fixture'],check=True)
    def case(name,hosts,args=['--dev'],extra=None,skill=False):
        folder=base/name;folder.mkdir();home=folder/'home';tools=folder/'tools';tools.mkdir()
        for relative,content in [('.claude/plugins/known_marketplaces.json','{"unrelated":{}}\n'),('.codex/config.toml','model = "preserve"\n')]:
            p=home/relative;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(content)
        for host in hosts:
            p=tools/host;p.write_text(HOST);p.chmod(0o755)
        if skill:(home/'.agents/skills/room').mkdir(parents=True)
        env=dict(os.environ,HOME=str(home),CLAUDE_CONFIG_DIR=str(home/'.claude'),CODEX_HOME=str(home/'.codex'),PATH=str(tools)+':/usr/bin:/bin:/usr/sbin:/sbin',CASE=str(folder));env.update(extra or {})
        p=subprocess.run([str(kit/'install.sh')]+args,cwd=base,env=env,text=True,capture_output=True)
        log=(folder/'log').read_text() if (folder/'log').exists() else ''
        return p,log,home
    for name,hosts in [('claude',['claude']),('codex',['codex']),('both',['claude','codex'])]:
        result,log,home=case(name,hosts,extra={'DOCTOR_EXIT':'1'})
        assert result.returncode==0,(name,result.stderr,result.stdout)
        assert 'Installed' in result.stdout and 'Next steps:' in result.stdout
        assert 'doctor' in log and 'bootstrap' in log and 'PATH' in result.stdout
        if 'claude' in hosts:assert any(p.read_text()=='{"unrelated":{}}\n' for p in (home/'.claude/plugins').glob('known_marketplaces.json.buzz-kit-backup.*'))
        if 'codex' in hosts:assert any(p.read_text()=='model = "preserve"\n' for p in (home/'.codex').glob('config.toml.buzz-kit-backup.*'))
    result,log,_=case('broken-doctor',['codex'],extra={'DOCTOR_EXIT':'127'});assert result.returncode!=0 and 'Installed' not in result.stdout
    result,log,_=case('neither',[]);assert result.returncode!=0 and 'bootstrap' not in log
    result,log,_=case('dry',['claude','codex'],['--dev','--dry-run']);assert result.returncode==0 and not log
    for collision in ['plugin','marketplace']:
        result,log,_=case(collision,['claude','codex'],extra={'COLLISION':collision});assert result.returncode!=0 and 'bootstrap' not in log
    result,log,_=case('skill',['codex'],skill=True);assert result.returncode!=0 and 'bootstrap' not in log
    result,log,_=case('failure',['claude','codex'],extra={'FAIL_HOST':'claude'});assert result.returncode!=0 and 'Installed' not in result.stdout and 'codex plugin marketplace add' not in log
    result,log,_=case('untagged',['codex'],[]);assert result.returncode!=0 and not log
    version=json.loads((kit/'plugin.json').read_text())['version'];subprocess.run(['git','-C',str(kit),'tag','v'+version],check=True)
    result,log,_=case('release-override',['codex'],[],extra={'BUZZ_KIT_BIN':'/fixture/dev'});assert result.returncode!=0 and 'bootstrap' not in log
    result,log,_=case('tagged',['claude','codex'],[]);assert result.returncode==0,result.stderr
    assert 'brklyn8900/buzz-agent-kit#v'+version in log and '--ref v'+version in log
    manifest=kit/'plugin.json';data=json.loads(manifest.read_text());data['version']='9.9.9';manifest.write_text(json.dumps(data))
    result,log,_=case('mismatch',['codex'],[]);assert result.returncode!=0 and 'bootstrap' not in log
print('PASS: both/single/no hosts, doctor setup warning, tag agreement, dry-run, collisions, backups and host failure')
