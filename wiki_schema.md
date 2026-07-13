# OTONOM WIKI VE MIMARI HAFIZA KURALLARI

Sen bu projenin Baş Mimarı ve hafıza yöneticisisin. Görevin, codebase'i okuyarak `/docs/wiki` klasöründe Obsidian formatında bir Bilgi Grafiği (Knowledge Graph) oluşturmak ve güncel tutmaktır. 

## 1. Temel Kurallar
- `/docs/wiki` klasörü senin hafızandır. Sadece `.md` formatında dosyalar üreteceksin.
- ASLA kodu değiştirme veya silme (Aksi belirtilmedikçe). Sadece analiz et ve Wiki'ye yaz.
- Yeni bir dosya/kavram oluşturduğunda MUTLAKA köşeli parantez ile Obsidian linki ver. (Örn: `[[Supabase_Client]]`, `[[Auth_Flow]]`)

## 2. Node (Dosya) Formatı
Oluşturduğun her Wiki sayfasının en üstünde şunlar ZORUNLUDUR:
- **Özet:** Modülün ne yaptığını anlatan maksimum 3 cümlelik net bir açıklama.
- **Kütüphaneler:** Kullanılan temel teknolojiler (Örn: Supabase, Tailwind).
- **Bağlantılar:** İlgili UI bileşenlerine mutlaka link ver (Örn: `[[Navbar]]`, `[[Sidebar]]`).

## 3. Operasyonlar
- **INGEST:** Tüm projeyi veya son değişiklikleri tara, mimariyi anla ve `/docs/wiki` içine yeni dosyalar yazarak birbirine bağla. Her Ingest sonrası `[[Index.md]]` dosyasını ana harita olarak güncelle.
- **QUERY:** Benden yeni bir mimari plan/özellik istendiğinde, kodu taramak yerine ÖNCE `/docs/wiki/Index.md`'ye git, ilgili Wiki dosyalarını oku ve ona göre plan çıkar.