import { test, expect } from '@playwright/test';

test.describe('Mock Backend Tests', () => {
  test.beforeEach(async ({ page }) => {
    // Listen for console logs
    page.on('console', msg => console.log(`[Browser Console] ${msg.type()}: ${msg.text()}`));
    
    // Inject mock backend
    await page.addInitScript(() => {
      console.log('Injecting Mock Backend...');

      let mockQueue: any[] = []; // Stateful queue

      // Helper function to route commands
      const handleCommand = (cmd: string, args: any, success: (data: any) => void, fail: (err: any) => void) => {
        // console.log(`[Mock Router] Handling ${cmd}`, args);
        
        switch (cmd) {
          case 'get_config':
            success({
              api: {
                provider: 'openai',
                api_key: 'sk-mock-key',
                model: 'gpt-4o',
                base_url: 'https://api.openai.com/v1'
              },
              project: {
                output_dir: '/tmp/output',
                source_lang: 'auto',
                target_lang: 'zh'
              },
              translation: {
                style: 'literary',
                context_size: 5,
                batch_size: 10
              },
              rag: {
                enabled: false,
                embedding_model: 'all-MiniLM-L6-v2',
                chunk_size: 512,
                chunk_overlap: 50,
                similarity_threshold: 0.7
              },
              advanced: {
                max_concurrency: 5,
                log_level: 'info'
              }
            });
            break;
            
          case 'save_config':
            setTimeout(() => success(null), 100);
            break;

          case 'get_glossary':
            success([
              { source: 'Kyogoku', target: '京极', context: 'Project Name' }
            ]);
            break;
            
          case 'add_files_to_queue':
             const paths = args.filePaths || [];
             const items = paths.map((p: string, i: number) => ({
                 id: `mock-${Date.now()}-${i}`,
                 file_path: p,
                 file_name: p.split(/[/\\]/).pop(), // Simple filename extraction
                 status: 'pending',
                 progress: 0,
                 error: null
             }));
             mockQueue = [...mockQueue, ...items];
             success(items);
             break;

          case 'get_file_queue':
             success(mockQueue);
             break;
             
          case 'clear_queue':
             mockQueue = [];
             success(null);
             break;

          case 'remove_from_queue':
             if (args.fileId) {
                mockQueue = mockQueue.filter(item => item.id !== args.fileId);
             }
             success(null);
             break;
             
          case 'translate_text':
             success(`Translated: ${args.key}`);
             break;
             
          case 'get_current_locale':
             success('en-US');
             break;
             
          case 'set_locale':
             success(null);
             break;

          case 'plugin:dialog|open':
             console.log('[Mock IPC] Opening dialog mock');
             // Return just paths as string[] or object depending on plugin version
             // @tauri-apps/plugin-dialog open() returns string | string[] | null
             // So we return the result directly.
             success(['/home/user/Documents/novel.txt']);
             break;
             
          case 'plugin:opener|open_path':
             success(null);
             break;
             
          case 'get_stats':
             success({
                 total_files: 10,
                 total_chars: 5000,
                 total_cost: 0.05,
                 history: []
             });
             break;

          case 'get_history':
             success([]);
             break;

          default:
            // console.warn(`[Mock IPC] Unhandled command: ${cmd}`);
            success(null);
        }
      };

      // Mock window.__TAURI_IPC__ (Low level)
      (window as any).__TAURI_IPC__ = async (message: any) => {
        const { cmd, callback, error, ...args } = message;
        console.log(`[Mock IPC] Command: ${cmd}`, message); 

        const success = (data: any) => {
           const cbName = `_${callback}`;
           // console.log(`[Mock IPC] Success callback: ${cbName} with data:`, data);
           if (typeof (window as any)[cbName] === 'function') {
               (window as any)[cbName](data);
           } else {
               // Also try finding it on window directly if not prefixed
               if (typeof (window as any)[callback] === 'function') {
                   (window as any)[callback](data);
               }
           }
        };
        
        const fail = (err: any) => {
            const errName = `_${error}`;
            // console.log(`[Mock IPC] Error callback: ${errName} with err:`, err);
            if (typeof (window as any)[errName] === 'function') {
                (window as any)[errName](err);
            }
        };

        handleCommand(cmd, args, success, fail);
      };

      // Mock window.__TAURI__ and window.__TAURI_INTERNALS__ (Global polyfill for v2/modules)
      const invokeMock = async (cmd: string, args: any = {}) => {
          console.log(`[Mock TAURI.invoke] ${cmd}`, args);
          return new Promise((resolve, reject) => {
              handleCommand(cmd, args, resolve, reject);
          });
      };

      (window as any).__TAURI__ = {
          invoke: invokeMock,
          transformCallback: (callback: any) => callback,
          promisified: (cmd: any) => Promise.resolve()
      };

      (window as any).__TAURI_INTERNALS__ = {
          invoke: invokeMock
      };
      
    });
  });

  test('Load Configuration', async ({ page }) => {
    await page.goto('/');
    
    // Check if API Key field is populated with mock data
    const providerSelect = page.locator('#api-provider');
    await expect(providerSelect).toHaveValue('openai');
    
    // Check Output Directory
    const outputDirInput = page.locator('#output-directory');
    await expect(outputDirInput).toHaveValue('/tmp/output');
  });

  test('Save Configuration Success', async ({ page }) => {
    await page.goto('/');
    
    // Wait for initial load
    await expect(page.locator('#api-provider')).toHaveValue('openai');
    
    const saveBtn = page.locator('#save-btn');
    await saveBtn.click();
    
    // Should show success toast
    const toast = page.locator('.bg-emerald-50'); 
    await expect(toast).toBeVisible();
    await expect(toast).toContainText('Configuration saved');
  });
  
  test('File Selection Mock', async ({ page }) => {
      await page.goto('/');
      
      // We can't easily trigger the native dialog via click if it uses system dialog
      // But we can invoke the command manually to test the UI response?
      // No, we want to test the UI interaction.
      // If we click the drop zone, it calls open().
      // Our mock open() returns files.
      
      const dropZone = page.locator('#drop-zone');
      await dropZone.click();
      
      // The app should receive files and call add_files_to_queue
      // Then render the queue.
      // We need to wait for the file item to appear.
      // The file item usually has the filename.
      // Mock returns: /home/user/Documents/novel.txt
      
      const fileItem = page.getByText('novel.txt');
      await expect(fileItem).toBeVisible();
      
      // Check status
      await expect(page.getByText('Pending')).toBeVisible();
  });

});
