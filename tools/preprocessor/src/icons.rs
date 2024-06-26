use std::{collections::HashMap, io::Cursor, path::Path};

use image::GenericImage;

fn extract(id: usize) -> Result<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>, crate::Error> {
	// aetherment -e --out - --outformat png ui/icon/062000/062040_hr1.tex
	let data = std::process::Command::new("aetherment")
		.args(["-e", "--out", "-", "--outformat", "png", &icon_path(id)])
		.stdout(std::process::Stdio::piped())
		.output()?;
	
	Ok(image::io::Reader::with_format(Cursor::new(data.stdout), image::ImageFormat::Png).decode()?.to_rgba8())
}

fn icon_path(id: usize) -> String {
	format!("ui/icon/{:0>3}000/{:0>6}_hr1.tex", id / 1000, id)
}

fn write_comp(dir: &Path, local_dir: &str, layers: Vec<Option<&str>>) -> Result<(), crate::Error> {
	use crate::tex_composite::*;
	
	let comp = Tex {
		layers: layers.into_iter().enumerate().map(|(i, color_option)| {
			Layer {
				name: format!("Layer{i}"),
				path: Path::Mod(format!("{local_dir}/{i}.tex")),
				blend: Blend::Normal,
				modifiers: if let Some(color_option) = color_option {
					vec![
						Modifier::Color {
							value: OptionOrStatic::Option(ColorOption(color_option.to_owned()))
						}
					]
				} else {
					Vec::new()
				}
			}
		}).rev().collect()
	};
	
	std::fs::write(dir.join("comp.tex.comp"), serde_json::to_string(&comp)?)?;
	
	Ok(())
}

enum Roles {
	Tank,
	Healer,
	Melee,
	Ranged,
	Caster,
	Crafter,
	Gatherer,
	Other,
}

impl Roles {
	pub fn option(&self) -> &'static str {
		match self {
			Roles::Tank => "Tank Color",
			Roles::Healer => "Healer Color",
			Roles::Melee => "Melee Color",
			Roles::Ranged => "Ranged Color",
			Roles::Caster => "Caster Color",
			Roles::Crafter => "Crafter Color",
			Roles::Gatherer => "Gatherer Color",
			Roles::Other => "No Job Color",
		}
	}
}

fn prepare_icon(icon: &mut image::ImageBuffer<image::Rgba<u8>, Vec<u8>>, alpha_resolver: impl Fn(&image::Rgba<u8>) -> u8) {
	let mut min = 255;
	let mut max = 0;
	for pixel in icon.pixels_mut().filter(|v| v[3] > 0) {
		let val = ((pixel.0[0] as f32 * 0.299) as u16 +
		           (pixel.0[1] as f32 * 0.587) as u16 +
		           (pixel.0[2] as f32 * 0.144) as u16).min(255) as u8;
		
		pixel[0] = val;
		pixel[1] = val;
		pixel[2] = val;
		pixel[3] = alpha_resolver(&pixel);
		
		min = min.min(val);
		max = max.max(val);
	}
	
	let scale = (max - min) as f32;
	for pixel in icon.pixels_mut().filter(|v| v[3] > 0) {
		let val = ((pixel.0[0] - min) as f32 / scale * 32.0) as u8 + (255 - 32);
		pixel[0] = val;
		pixel[1] = val;
		pixel[2] = val;
	}
}

fn add_border(icon: &mut image::ImageBuffer<image::Rgba<u8>, Vec<u8>>) {
	let (w, h) = (icon.width(), icon.height());
	let mut new = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::new(w, h);
	for x in 0..w {
		for y in 0..h {
			let mut max = 0;
			for x2 in (x - 2).max(0)..=(x + 2).min(w - 1) {
				for y2 in (y - 2).max(0)..=(y + 2).min(h - 1) {
					let dist = ((x2 as f32 - x as f32).powi(2) + (y2 as f32 - y as f32).powi(2)).sqrt();
					max = max.max((icon.get_pixel(x2, y2)[3] as f32 * (1.0 - (dist - 2.0).clamp(0.0, 1.0))) as u8)
				}
			}
			
			let pixel = new.get_pixel_mut(x, y);
			pixel[0] = 12;
			pixel[1] = 12;
			pixel[2] = 12;
			pixel[3] = max;
		}
	}
	
	image::imageops::overlay(&mut new, icon, 0, 0);
	*icon = new;
}

fn center(icon: &mut image::ImageBuffer<image::Rgba<u8>, Vec<u8>>) {
	let (mut min_x, mut min_y, mut max_x, mut max_y) = (9999, 9999, 0, 0);
	let (w, h) = (icon.width(), icon.height());
	for x in 0..w {
		for y in 0..h {
			if icon.get_pixel(x, y)[3] > 50 {
				min_x = min_x.min(x);
				min_y = min_y.min(y);
				max_x = max_x.max(x);
				max_y = max_y.max(y);
			}
		}
	}
	
	let offset_x = (w as i32 - (max_x + min_x) as i32) / 2;
	let offset_y = (h as i32 - (max_y + min_y) as i32) / 2;
	let mut new = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::new(w, h);
	for x in offset_x.abs().max(0)..(w as i32 - offset_x.abs()) {
		for y in offset_y.abs()..(h as i32 - offset_y.abs()) {
			new.put_pixel(x as u32, y as u32, icon.get_pixel((x - offset_x) as u32, (y - offset_y) as u32).to_owned())
		}
	}
	
	*icon = new;
}

pub fn job_icons(target_root: &Path) -> Result<HashMap<(&str, &str), HashMap<String, String>>, crate::Error> {
	let icon_roles = HashMap::from([
		(1, Roles::Tank), // gla
		(2, Roles::Melee), // pgl
		(3, Roles::Tank), // mrd
		(4, Roles::Melee), // lnc
		(5, Roles::Ranged), // arc
		(6, Roles::Healer), // cnj
		(7, Roles::Caster), // thm
		(8, Roles::Crafter), // crp
		(9, Roles::Crafter), // bsm
		(10, Roles::Crafter), // arm
		(11, Roles::Crafter), // gsm
		(12, Roles::Crafter), // ltw
		(13, Roles::Crafter), // wvr
		(14, Roles::Crafter), // alc
		(15, Roles::Crafter), // cul
		(16, Roles::Gatherer), // min
		(17, Roles::Gatherer), // bot
		(18, Roles::Gatherer), // fsh
		(19, Roles::Tank), // pld
		(20, Roles::Melee), // mnk
		(21, Roles::Tank), // war
		(22, Roles::Melee), // drg
		(23, Roles::Ranged), // brd
		(24, Roles::Healer), // whm
		(25, Roles::Caster), // blm
		(26, Roles::Caster), // acn
		(27, Roles::Caster), // smn
		(28, Roles::Healer), // sch
		(29, Roles::Melee), // rog
		(30, Roles::Melee), // nin
		(31, Roles::Ranged), // mch
		(32, Roles::Tank), // drk
		(33, Roles::Healer), // ast
		(34, Roles::Melee), // sam
		(35, Roles::Caster), // rdm
		(36, Roles::Caster), // blu
		(37, Roles::Tank), // gnb
		(38, Roles::Ranged), // dnc
		(39, Roles::Melee), // rpr
		(40, Roles::Healer), // sge
		(41, Roles::Melee), // vpr
		(42, Roles::Caster), // pct
		(43, Roles::Other), // chocobo
		(44, Roles::Other), // carbuncle
		(45, Roles::Other), // free slot
	]);
	
	// load in extra assets
	let asset_dir = target_root.join("assets").join("job icon backgrounds");
	let opt = resvg::usvg::Options::default();
	let font = resvg::usvg::fontdb::Database::new();
	
	let rounded_64 = {
		let tree = resvg::usvg::Tree::from_data(&std::fs::read(asset_dir.join("rounded_64.svg"))?, &opt, &font)?;
		let mut pixmap = resvg::tiny_skia::Pixmap::new(64, 64).ok_or("Failed creating pixmap with specified size")?;
		resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());
		image::RgbaImage::from_vec(64, 64, pixmap.data().to_owned()).ok_or("Failed loading in rounded_64.svg")?
	};
	
	let square_64 = {
		let tree = resvg::usvg::Tree::from_data(&std::fs::read(asset_dir.join("square_64.svg"))?, &opt, &font)?;
		let mut pixmap = resvg::tiny_skia::Pixmap::new(64, 64).ok_or("Failed creating pixmap with specified size")?;
		resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());
		image::RgbaImage::from_vec(64, 64, pixmap.data().to_owned()).ok_or("Failed loading in square_64.svg")?
	};
	
	let action_80 = image::open(asset_dir.join("action_80.png"))?.into_rgba8();
	
	// do the thing
	let mut files = HashMap::new();
	let files_root = target_root.join("files");
	for i in 1..=99 {
		let content_id = 062000 + i;
		let party_id   = 062100 + i;
		let macro_id   = 062800 + i;
		let Ok(mut icon_56) = extract(content_id) else {break};
		let color = icon_roles[&i].option();
		
		// greyscale, shove it into the 191-255 range, and do some alpha stuff
		prepare_icon(&mut icon_56, |pixel| (((pixel[3] as f32 / 255.0).max(0.75) - 0.75) * 4.0 * 255.0) as u8);
		center(&mut icon_56);
		
		// (nearly)black border
		let mut icon_border_56 = icon_56.clone();
		add_border(&mut icon_border_56);
		
		// icon_border_80 = image::imageops::blur(&icon_border_80, 2.0);
		image::imageops::overlay(&mut icon_border_56, &icon_56, 0, 0);
		
		// glow
		let mut icon_glow_56 = image::imageops::blur(&icon_56, 4.0);
		for pixel in icon_glow_56.pixels_mut() {
			pixel[0] = 255;
			pixel[1] = 255;
			pixel[2] = 255;
			pixel[3] = (pixel[3] as u16 * 4).min(255) as u8;
		}
		
		//// save em all
		let mut icon_faded2_56 = icon_56.clone();
		for pixel in icon_faded2_56.pixels_mut() {pixel[3] = (pixel[3] as f32 * 0.3) as u8;}
		
		let content_path = icon_path(content_id);
		{ // content glow
			let local_dir = format!("{}/Job Icons Content/Glow", content_path);
			let dir = files_root.join(&local_dir);
			_ = std::fs::create_dir_all(&dir);
			
			crate::save_tex(56, 56, icon_glow_56.as_raw(), &dir.join("0.tex"))?;
			crate::save_tex(56, 56, icon_56.as_raw(), &dir.join("1.tex"))?;
			write_comp(&dir, &local_dir, vec![Some(color), None])?;
			files.entry(("Job Icons Content", "Glow")).or_insert_with(|| HashMap::new()).insert(format!("{content_path}.comp"), format!("{local_dir}/comp.tex.comp"));
		}
		
		{ // content border
			let local_dir = format!("{}/Job Icons Content/Border", content_path);
			let dir = files_root.join(&local_dir);
			_ = std::fs::create_dir_all(&dir);
			
			crate::save_tex(56, 56, icon_border_56.as_raw(), &dir.join("0.tex"))?;
			crate::save_tex(56, 56, icon_faded2_56.as_raw(), &dir.join("1.tex"))?;
			write_comp(&dir, &local_dir, vec![Some(color), None])?;
			files.entry(("Job Icons Content", "Border")).or_insert_with(|| HashMap::new()).insert(format!("{content_path}.comp"), format!("{local_dir}/comp.tex.comp"));
		}
		
		// party list icons
		let icon_64 = image::imageops::resize(&icon_56, 64, 64, image::imageops::FilterType::CatmullRom);
		let icon_border_64 = image::imageops::resize(&icon_border_56, 64, 64, image::imageops::FilterType::CatmullRom);
		let icon_glow_64 = image::imageops::resize(&icon_glow_56, 64, 64, image::imageops::FilterType::CatmullRom);
		
		let mut icon_faded_64 = icon_64.clone();
		for pixel in icon_faded_64.pixels_mut() {pixel[3] = (pixel[3] as f32 * 0.8) as u8;}
		let mut icon_faded2_64 = icon_64.clone();
		for pixel in icon_faded2_64.pixels_mut() {pixel[3] = (pixel[3] as f32 * 0.3) as u8;}
		
		let party_path = icon_path(party_id);
		{ // party glow
			let local_dir = format!("{}/Job Icons Party List/Glow", party_path);
			let dir = files_root.join(&local_dir);
			_ = std::fs::create_dir_all(&dir);
			
			crate::save_tex(64, 64, icon_glow_64.as_raw(), &dir.join("0.tex"))?;
			crate::save_tex(64, 64, icon_64.as_raw(), &dir.join("1.tex"))?;
			write_comp(&dir, &local_dir, vec![Some(color), None])?;
			files.entry(("Job Icons Party List", "Glow")).or_insert_with(|| HashMap::new()).insert(format!("{party_path}.comp"), format!("{local_dir}/comp.tex.comp"));
		}
		
		{ // party border
			let local_dir = format!("{}/Job Icons Party List/Border", party_path);
			let dir = files_root.join(&local_dir);
			_ = std::fs::create_dir_all(&dir);
			
			crate::save_tex(64, 64, icon_border_64.as_raw(), &dir.join("0.tex"))?;
			crate::save_tex(64, 64, icon_faded2_64.as_raw(), &dir.join("1.tex"))?;
			write_comp(&dir, &local_dir, vec![Some(color), None])?;
			files.entry(("Job Icons Party List", "Border")).or_insert_with(|| HashMap::new()).insert(format!("{party_path}.comp"), format!("{local_dir}/comp.tex.comp"));
		}
		
		{ // party square
			let local_dir = format!("{}/Job Icons Party List/Square", party_path);
			let dir = files_root.join(&local_dir);
			_ = std::fs::create_dir_all(&dir);
			
			crate::save_tex(64, 64, square_64.as_raw(), &dir.join("0.tex"))?;
			crate::save_tex(64, 64, icon_border_64.as_raw(), &dir.join("1.tex"))?;
			crate::save_tex(64, 64, icon_faded_64.as_raw(), &dir.join("2.tex"))?;
			write_comp(&dir, &local_dir, vec![Some(color), Some(color), None])?;
			files.entry(("Job Icons Party List", "Square")).or_insert_with(|| HashMap::new()).insert(format!("{party_path}.comp"), format!("{local_dir}/comp.tex.comp"));
		}
		
		{ // party square
			let local_dir = format!("{}/Job Icons Party List/Rounded", party_path);
			let dir = files_root.join(&local_dir);
			_ = std::fs::create_dir_all(&dir);
			
			crate::save_tex(64, 64, rounded_64.as_raw(), &dir.join("0.tex"))?;
			crate::save_tex(64, 64, icon_border_64.as_raw(), &dir.join("1.tex"))?;
			crate::save_tex(64, 64, icon_faded_64.as_raw(), &dir.join("2.tex"))?;
			write_comp(&dir, &local_dir, vec![Some(color), Some(color), None])?;
			files.entry(("Job Icons Party List", "Rounded")).or_insert_with(|| HashMap::new()).insert(format!("{party_path}.comp"), format!("{local_dir}/comp.tex.comp"));
		}
		
		// macro icons
		let icon_80 = image::imageops::resize(&icon_56, 80, 80, image::imageops::FilterType::CatmullRom);
		let icon_border_80 = image::imageops::resize(&icon_border_56, 80, 80, image::imageops::FilterType::CatmullRom);
		let icon_glow_80 = image::imageops::resize(&icon_glow_56, 80, 80, image::imageops::FilterType::CatmullRom);
		
		let mut icon_faded_80 = icon_80.clone();
		for pixel in icon_faded_80.pixels_mut() {pixel[3] = (pixel[3] as f32 * 0.8) as u8;}
		let mut icon_faded2_80 = icon_80.clone();
		for pixel in icon_faded2_80.pixels_mut() {pixel[3] = (pixel[3] as f32 * 0.3) as u8;}
		
		let macro_path = icon_path(macro_id);
		{ // macro glow
			let local_dir = format!("{}/Job Icons Macro/Glow", macro_path);
			let dir = files_root.join(&local_dir);
			_ = std::fs::create_dir_all(&dir);
			
			crate::save_tex(80, 80, icon_glow_80.as_raw(), &dir.join("0.tex"))?;
			crate::save_tex(80, 80, icon_80.as_raw(), &dir.join("1.tex"))?;
			write_comp(&dir, &local_dir, vec![Some(color), None])?;
			files.entry(("Job Icons Macro", "Glow")).or_insert_with(|| HashMap::new()).insert(format!("{macro_path}.comp"), format!("{local_dir}/comp.tex.comp"));
		}
		
		{ // macro border
			let local_dir = format!("{}/Job Icons Macro/Border", macro_path);
			let dir = files_root.join(&local_dir);
			_ = std::fs::create_dir_all(&dir);
			
			crate::save_tex(80, 80, icon_border_80.as_raw(), &dir.join("0.tex"))?;
			crate::save_tex(80, 80, icon_faded2_80.as_raw(), &dir.join("1.tex"))?;
			write_comp(&dir, &local_dir, vec![Some(color), None])?;
			files.entry(("Job Icons Macro", "Border")).or_insert_with(|| HashMap::new()).insert(format!("{macro_path}.comp"), format!("{local_dir}/comp.tex.comp"));
		}
		
		{ // macro full
			let local_dir = format!("{}/Job Icons Macro/Full", macro_path);
			let dir = files_root.join(&local_dir);
			_ = std::fs::create_dir_all(&dir);
			
			crate::save_tex(80, 80, action_80.as_raw(), &dir.join("0.tex"))?;
			crate::save_tex(80, 80, icon_border_80.as_raw(), &dir.join("1.tex"))?;
			crate::save_tex(80, 80, icon_faded_80.as_raw(), &dir.join("2.tex"))?;
			write_comp(&dir, &local_dir, vec![Some(color), Some(color), None])?;
			files.entry(("Job Icons Macro", "Full")).or_insert_with(|| HashMap::new()).insert(format!("{macro_path}.comp"), format!("{local_dir}/comp.tex.comp"));
		}
		
		// TODO: nameplate type 1
	}
	
	Ok(files)
}

pub fn tribe_icons(target_root: &Path) -> Result<HashMap<String, String>, crate::Error> {
	let mut files = HashMap::new();
	let files_root = target_root.join("files");
	for id in 061901..=061959 {
		let Ok(mut icon) = extract(id) else {break};
		
		prepare_icon(&mut icon, |pixel| if pixel[3] > 200 {((pixel[0] as f32 * 4.0) - 512.0).clamp(0.0, 255.0) as u8} else {0});
		add_border(&mut icon);
		
		let local_path = icon_path(id);
		let dir = files_root.join(&local_path);
		_ = std::fs::create_dir_all(&dir);
		
		crate::save_tex(64, 64, icon.as_raw(), &dir.join("0.tex"))?;
		write_comp(&dir, &local_path, vec![Some("Foreground Color")])?;
		files.insert(format!("{local_path}.comp"), format!("{local_path}/comp.tex.comp"));
	}
	
	Ok(files)
}

pub fn silver_bordered(target_root: &Path) -> Result<HashMap<String, String>, crate::Error> {
	let mut files = HashMap::new();
	let files_root = target_root.join("files");
	for id in 061751..=061874 {
		if id == 061800 {continue};
		let Ok(mut icon) = extract(id) else {continue};
		
		let z = || -> image::Rgba<u8> {[0, 0, 0, 0].into()};
		let s = icon.width();
		for x in 0..s {
			for y in 0..6 {
				icon.put_pixel(x, y, z());
				icon.put_pixel(x, s - 1 - y, z());
				icon.put_pixel(y, x, z());
				icon.put_pixel(s - 1 - y, x, z());
			}
		}
		
		for x in 0..4 {
			for y in 0..(4 - x) {
				icon.put_pixel(6 + x, 6 + y, z());
				icon.put_pixel(s - 7 - x, 6 + y, z());
				icon.put_pixel(6 + x, s - 7 - y, z());
				icon.put_pixel(s - 7 - x, s - 7 - y, z());
			}
		}
		
		// icon = image::imageops::resize(&icon.sub_image(6, 6, 52, 52).to_image(), 64, 64, image::imageops::FilterType::CatmullRom);
		
		let local_path = icon_path(id);
		let path = files_root.join(&local_path);
		_ = std::fs::create_dir_all(&path.parent().unwrap());
		
		crate::save_tex(64, 64, icon.as_raw(), &path)?;
		files.insert(local_path.clone(), local_path);
	}
	
	Ok(files)
}

pub fn shop_icons(target_root: &Path) -> Result<HashMap<String, String>, crate::Error> {
	let mut files = HashMap::new();
	let files_root = target_root.join("files");
	for id in 060101..=060199 {
		if id == 060158 {continue}; // some quest marker icon, why??
		let Ok(mut icon) = extract(id) else {continue};
		
		let mut new = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::new(40, 40);
		image::imageops::overlay(&mut new, &icon.sub_image(4, 4, 32, 32).to_image(), 4, 4);
		icon = new;
		
		prepare_icon(&mut icon, |pixel| if pixel[3] > 240 {(255.0 - ((pixel[0] as f32 * 30.0) - 2048.0).clamp(0.0, 255.0)) as u8} else {0});
		add_border(&mut icon);
		
		let local_path = icon_path(id);
		let path = files_root.join(&local_path);
		_ = std::fs::create_dir_all(&path.parent().unwrap());
		
		// icon.save(files_root.join(format!("{local_path}.png")))?;
		crate::save_tex(40, 40, icon.as_raw(), &path)?;
		files.insert(local_path.clone(), local_path);
	}
	
	Ok(files)
}

/*
ranges:
062001-062099 = job content
062101-062199 = job party
062226-062299 = job nameplate type 1 (+25 of other ids)
062301-062399 = job glow (only the base classes)
062401-062499 = job glow (only the jobs)
062801-062899 = job macro

061751-061874 = silver bordered
061901-061959 = beast tribe
060101-060199 = shop icons
*/