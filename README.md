# Commit 1
![Chat Application Demo](img/commit_1.png)

Seperti yang ditunjukkan pada gambar, saya telah menjalankan satu instance server dan tiga instance client secara bersamaan untuk mendemonstrasikan fungsionalitas aplikasi chat. Server berhasil menerima koneksi dari ketiga client dan menampilkan alamat IP serta port dari setiap koneksi yang masuk. Ketika pesan dikirim dari salah satu client, server akan menerimanya dan langsung menyebarkannya ke semua client yang terhubung, memungkinkan komunikasi real-time antar pengguna.

Log server menampilkan informasi detail tentang aktivitas koneksi, termasuk waktu koneksi dibuat dan pesan-pesan yang diterima dari setiap client. Setiap client dapat melihat pesan yang dikirim oleh client lain berkat mekanisme broadcasting yang diimplementasikan menggunakan channel Tokio. Pendekatan asinkron memungkinkan server untuk menangani banyak koneksi secara efisien tanpa memblokir thread utama, sehingga aplikasi tetap responsif bahkan ketika jumlah client bertambah.

# Commit 2: Modifying Port
![alt text](img/commit_2.png)

Untuk mengubah port aplikasi dari 2000 menjadi 8080, saya perlu memodifikasi dua file dalam kode:

1. Di file `server.rs`, saya mengubah alamat binding server dari "127.0.0.1:2000" menjadi "127.0.0.1:8080" pada baris yang mendefinisikan TcpListener. Ini mengubah port yang didengarkan oleh server.

2. Di file `client.rs`, saya mengubah URI koneksi dari "ws://127.0.0.1:2000" menjadi "ws://127.0.0.1:8080" pada kode yang membuat koneksi client. Perhatikan bahwa koneksi menggunakan protokol WebSocket yang ditandai dengan awalan "ws://" pada URI.

Setelah modifikasi, aplikasi masih berjalan dengan baik. Server sekarang mendengarkan pada port 8080 dan client terhubung ke port yang sama. Komunikasi antara client dan server tetap berfungsi seperti sebelumnya, memungkinkan pesan dikirim dan diterima antara semua client yang terhubung.

# Commit 3: Small Changes, Add IP and Port
![Chat with Sender Info](img/commit_3.png)
Untuk meningkatkan fungsionalitas aplikasi chat, saya melakukan modifikasi pada kode sehingga pesan yang dikirim akan disertai dengan informasi pengirim (alamat IP dan port):

1. Di file `server.rs`, saya menambahkan informasi alamat pengirim pada pesan sebelum disebarkan:
   ```rust
   // Add sender information to message
   let formatted_msg = format!("[{}] {}", addr, text);
   
   // Broadcast to all subscribers
   let _ = bcast_tx.send(formatted_msg);
   ```

2. Di file `client.rs` tidak diperlukan perubahan khusus karena pesan yang diterima sudah diformat dengan informasi pengirim oleh server.

Dengan modifikasi ini, setiap kali pesan dikirim, penerima dapat melihat siapa (alamat IP dan port) yang mengirim pesan tersebut. Hal ini memudahkan pengguna untuk mengidentifikasi sumber pesan dalam percakapan grup, terutama ketika ada banyak klien yang terhubung secara bersamaan. Pendekatan ini menambahkan konteks pada percakapan tanpa perlu mengimplementasikan sistem login atau nama pengguna. Selain itu, solusi ini juga sangat efisien dari segi sumber daya karena hanya memerlukan sedikit tambahan pemrosesan pada server tanpa menambah beban pada klien. Implementasi ini juga dapat dengan mudah diperluas di masa depan, misalnya dengan menambahkan fitur nickname atau avatar untuk identifikasi pengguna yang lebih personal.