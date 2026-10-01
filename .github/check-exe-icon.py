import pefile, sys
pe = pefile.PE(sys.argv[1])
kinds = {e.id for e in pe.DIRECTORY_ENTRY_RESOURCE.entries}
# 3 = the icon images, 14 = the icon group, 16 = the version information
assert {3, 14, 16} <= kinds, f"the .exe lacks its icon resources: {sorted(kinds)}"
print("icon and version resources are in", sys.argv[1])
