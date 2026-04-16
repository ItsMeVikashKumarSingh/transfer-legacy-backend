import os
import re

handlers_dir = r"c:\Users\vikas\OneDrive\Desktop\projects\Transfer Legacy\Transfer-Legacy-backend\crates\api\src\handlers"

RID_DEF = """
    let rid = request_id_ext
        .map(|Extension(id)| crate::middleware::request_id::request_id_string(&id))
        .unwrap_or_else(|| "unknown".to_string());"""

def fix_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    new_content = content

    # 1. Signature fix (if not already done)
    # Match various ways RequestId is passed
    patterns_to_fix = [
        r'Extension\(request_id\):\s*Extension<tower_http::request_id::RequestId>,',
        r'Extension\(request_id\):\s*Extension<tower_http::request_id::RequestId>\)',
        r'request_id:\s*Extension<tower_http::request_id::RequestId>,',
        r'request_id:\s*Extension<tower_http::request_id::RequestId>\)'
    ]
    for p in patterns_to_fix:
        new_content = re.sub(p, 'request_id_ext: Option<Extension<tower_http::request_id::RequestId>>,', new_content)
    
    # Fix potential double commas from the replacement
    new_content = new_content.replace(',,', ',')
    new_content = new_content.replace(',)', ')')

    # 2. RID Definition Insertion
    # We find where a function has request_id_ext but no definition for rid
    # We'll split the file by function definitions
    
    # This regex finds the opening brace of a function that takes request_id_ext
    # It accounts for multi-line signatures
    func_pattern = re.compile(r'(pub async fn .*?request_id_ext:.*?\)[\s\S]*?->[\s\S]*?Result<[\s\S]*?>\s*\{)')
    
    parts = []
    last_end = 0
    matches_found = 0
    for match in func_pattern.finditer(new_content):
        # Add everything before the match
        parts.append(new_content[last_end:match.end()])
        
        # Check if following content already has rid definition
        remaining = new_content[match.end():]
        if 'let rid =' not in remaining[:200]:
            parts.append(RID_DEF)
            matches_found += 1
        
        last_end = match.end()
    
    parts.append(new_content[last_end:])
    final_content = "".join(parts)

    # 3. Usage fixing
    # Replace &request_id with &rid
    final_content = final_content.replace('&request_id', '&rid')
    # If request_id is used alone (e.g. in webauthn.rs)
    final_content = re.sub(r'(\W)request_id(\W)', r'\1&rid\2', final_content) 
    # Clean up potentially doubled ampersands from the above rule
    final_content = final_content.replace('&&rid', '&rid')
    final_content = final_content.replace('crate::middleware::request_id::request_id_string(&rid)', 'rid')
    final_content = final_content.replace('request_id: rid', 'request_id: rid.clone()') # Avoid move errors

    if final_content != content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(final_content)
        print(f"Fixed {filepath} ({matches_found} rid insertions)")

for root, dirs, files in os.walk(handlers_dir):
    for file in files:
        if file.endswith(".rs"):
            fix_file(os.path.join(root, file))
print("Done.")
