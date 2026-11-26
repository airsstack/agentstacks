# gen.py NAME EVENT:MATCHER ... -> NAME.hooks.json ; matcher "-" = no matcher; each group appends "EVENT matcher" to @OUT@/fired.txt
import json, sys
name=sys.argv[1]; h={"hooks":{}}
for spec in sys.argv[2:]:
    ev, m = spec.split(":",1)
    g={"hooks":[{"type":"command","command":"cat >/dev/null; echo '%s %s' >> @OUT@/fired.txt" % (ev, m)}]}
    if m != "-":
        try: g["matcher"]=json.loads(m)
        except Exception: g["matcher"]=m
    h["hooks"].setdefault(ev,[]).append(g)
json.dump(h, open(name+".hooks.json","w"), indent=1)
