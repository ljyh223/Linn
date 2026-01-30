use relm4::prelude::*;
use gtk::prelude::*;
use reqwest::header::{HeaderMap, USER_AGENT};
use sha2::{Sha256, Digest};

pub struct AsyncImage {
    texture: Option<gtk::gdk::Texture>,
    placeholder_texture: Option<gtk::gdk::Texture>,
    is_loading: bool,
    is_error: bool,
    class_name: String,
    stack: gtk::Stack,
}

#[derive(Debug)]
pub enum AsyncImageMsg {
    Load(String),
    LoadSuccess(gtk::gdk::Texture),
    LoadError,
}

#[relm4::component(pub)]
impl SimpleComponent for AsyncImage {
    type Init = (Option<String>, Option<String>, String); 
    type Input = AsyncImageMsg;
    type Output = ();

       view! {
        #[root]
        gtk::Box {
            add_css_class: &model.class_name,

            #[name(stack)]
            gtk::Stack {
                set_transition_type: gtk::StackTransitionType::Crossfade,
                set_transition_duration: 500,
            }
        }
    }

    fn init(init: Self::Init, _root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let (url, placeholder_path, class_name) = init;

        // 尝试加载本地占位图
        let placeholder_texture = placeholder_path.and_then(|path| {
            gtk::gdk::Texture::from_filename(path).ok()
        });

        let mut model = AsyncImage {
            texture: None,
            placeholder_texture,
            is_loading: url.is_some(),
            is_error: false,
            class_name,
            stack: gtk::Stack::default(), // 临时创建，后面会替换
        };

        let widgets = view_output!();

        // 替换为实际的stack
        model.stack.clone_from(&widgets.stack);

        // 创建占位图
        let mut placeholder_builder = gtk::Image::builder();
        if let Some(texture) = model.placeholder_texture.as_ref() {
            placeholder_builder = placeholder_builder.paintable(texture.upcast_ref::<gtk::gdk::Paintable>());
        } else {
            placeholder_builder = placeholder_builder.icon_name("image-x-generic-symbolic");
        }
        let placeholder = placeholder_builder
            .css_classes(["placeholder-style"])
            .build();

        // 创建错误图
        let error = gtk::Image::builder()
            .icon_name("image-missing-symbolic")
            .css_classes(["error-style"])
            .build();

        // 创建真实图片容器
        let image = gtk::Image::new();

        // 添加到Stack
        widgets.stack.add_named(&placeholder, Some("placeholder"));
        widgets.stack.add_named(&error, Some("error"));
        widgets.stack.add_named(&image, Some("image"));

        // 设置初始可见的子节点
        if model.is_loading {
            widgets.stack.set_visible_child_name("placeholder");
        } else if model.is_error {
            widgets.stack.set_visible_child_name("error");
        } else {
            widgets.stack.set_visible_child_name("image");
        }

        if let Some(url_str) = url {
            sender.input(AsyncImageMsg::Load(url_str));
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AsyncImageMsg::Load(url) => {
                if url.is_empty() { return; }
                self.is_loading = true;
                self.is_error = false;
                self.stack.set_visible_child_name("placeholder");

                let sender_clone = sender.clone();
                relm4::spawn(async move {
                    // 使用带缓存逻辑的加载函数
                    match load_image_smart(url).await {
                        Ok(texture) => sender_clone.input(AsyncImageMsg::LoadSuccess(texture)),
                        Err(_) => sender_clone.input(AsyncImageMsg::LoadError),
                    }
                });
            }
            AsyncImageMsg::LoadSuccess(texture) => {
                self.is_loading = false;
                self.texture = Some(texture);

                // 更新image widget的paintable
                if let Some(child) = self.stack.child_by_name("image") {
                    if let Some(image) = child.downcast_ref::<gtk::Image>() {
                        image.set_paintable(Some(self.texture.as_ref().unwrap().upcast_ref::<gtk::gdk::Paintable>()));
                    }
                }
                self.stack.set_visible_child_name("image");
            }
            AsyncImageMsg::LoadError => {
                self.is_loading = false;
                self.is_error = true;
                self.stack.set_visible_child_name("error");
            }
        }
    }
}

// --- 智能加载逻辑：磁盘缓存 + 网络请求 ---
async fn load_image_smart(url: String) -> Result<gtk::gdk::Texture, Box<dyn std::error::Error + Send + Sync>> {
    // 1. 确定缓存路径
    let cache_dir = dirs::cache_dir()
        .ok_or("无法获取缓存目录")?
        .join("linn/images");
    
    if !cache_dir.exists() {
        std::fs::create_dir_all(&cache_dir)?;
    }

    // 使用 URL 的 SHA256 哈希作为文件名
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let file_name = hex::encode(hasher.finalize());
    let cache_path = cache_dir.join(file_name);

    // 2. 检查缓存是否存在
    let bytes_data = if cache_path.exists() {
        // 命中缓存：直接读磁盘
        tokio::fs::read(&cache_path).await?
    } else {
        // 未命中：网络下载
        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, "Mozilla/5.0 (X11; Linux x86_64; rv:147.0) Gecko/20100101 Firefox/147.0".parse()?);

        let resp = client.get(url)
            .headers(headers)
            .send()
            .await?
            .bytes()
            .await?;
        
        let data = resp.to_vec();
        // 存入缓存
        tokio::fs::write(&cache_path, &data).await?;
        data
    };

    // 3. 转换为纹理
    let gbytes = gtk::glib::Bytes::from(&bytes_data);
    let texture = gtk::gdk::Texture::from_bytes(&gbytes)
        .map_err(|e| format!("Texture Error: {}", e))?;

    Ok(texture)
}