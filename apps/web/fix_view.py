import re
path = r'D:\\Android\\AtlasRemote\\apps\\web\\src\\views\\RemoteView.vue'
with open(path, 'r', encoding='utf-8') as f:
    lines = f.readlines()
for i, line in enumerate(lines):
    if 'statusColor' in line and 'computed' in line:
        lines[i] = 'const statusColor = computed(() => { const s = appStore.connectionStatus; return s === '"'"'connected'"'"' ? '"'"'online'"'"' : s === '"'"'connecting'"'"' ? '"'"'connecting'"'"' : s === '"'"'error'"'"' ? '"'"'error'"'"' : '"'"'offline'"'"' })\n'
    if 'statusText' in line and 'computed' in line:
        lines[i] = 'const statusText = computed(() => { const s = appStore.connectionStatus; return s === '"'"'connected'"'"' ? '"'"'Connected'"'"' : s === '"'"'connecting'"'"' ? '"'"'Connecting...'"'"' : s === '"'"'error'"'"' ? '"'"'Failed'"'"' : '"'"'Disconnected'"'"' })\n'
with open(path, 'w', encoding='utf-8') as f:
    f.writelines(lines)
print('Fixed')
