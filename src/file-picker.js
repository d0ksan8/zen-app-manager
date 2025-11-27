// File Picker Module
const { open } = window.__TAURI__.dialog;

const browseBtn = document.getElementById("browse-btn");
const commandInput = document.getElementById("app-command");
const nameInput = document.getElementById("app-name");

// File Picker Logic - Optional for users who want to browse
if (browseBtn) {
    browseBtn.onclick = async () => {
        try {
            const selected = await open({
                multiple: false,
                filters: [{
                    name: 'Applications',
                    extensions: ['exe', 'lnk', 'sh', 'desktop', 'AppImage', 'bat', 'cmd']
                }]
            });

            if (selected) {
                commandInput.value = selected;

                // Auto-fill name if empty
                if (!nameInput.value) {
                    const filename = selected.split(/[\\/]/).pop();
                    const name = filename.split('.').slice(0, -1).join('.') || filename;
                    nameInput.value = name.charAt(0).toUpperCase() + name.slice(1);
                }
            }
        } catch (error) {
            console.error("Failed to open file dialog:", error);
        }
    };
}
