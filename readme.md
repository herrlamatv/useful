## Useful stuff
This **repo** brings useful stuff I use on a daily basis<br>Here you can find plenty of stuff I use, such as:<br>**Scripts** for applications & more <br><br>
## yt-dlp: (Simple)
- At first, you need [yt-dlp.exe](https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe) (download by clicking)
- Then: Create a folder on your **Desktop** named: **dw**
- Place the [yt-dlp.exe](https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe) into the **Desktop\dw** (folder)

<br>
<br>

### You can download the Video in "1080p" or in "Best" (original)
Note: 1080p limits the height of the video, so if you want to download shorts form content, like "Reels", "Shorts" or "Tiktoks", you will need to download them via **"Best"**<br>"BEST" makes it so the program saves the video in original quallity, this means <br>that if the video is in 4k (3840:2160), its much larger then you even need it.<br>Why: 

| Video        |format| Resolution (p) |
|--------------|---|----------------|
| Longform     |16:9| 1920:1080      |
| Shorts/Reels |9:16| 1080:1920      

| 1080p                                                                                |Best Quality/Shorts|
|--------------------------------------------------------------------------------------|---|
| [dw-1080p.bat](https://github.com/herrlamatv/useful/blob/master/yt-dlp/dw-1080p.bat) |[dw-best.bat](https://github.com/herrlamatv/useful/blob/master/yt-dlp/dw-best.bat)
| [[Download]](https://github.com/herrlamatv/useful/releases/download/ytdlp.simple.files.1/dw-1080p.bat)                                                                       |[[Download]](https://github.com/herrlamatv/useful/releases/download/ytdlp.simple.files.1/dw-best.bat)

## 1080p
- Download [[this]](https://github.com/herrlamatv/useful/releases/download/ytdlp.simple.files.1/dw-1080p.bat)
- **Place it on the same directory as the _yt-dlp.exe_**
- start it
- A command line (cmd) will appear
- paste your link and wait

## Best
- Same as the '[1080p](https://github.com/herrlamatv/useful#1080p)',<br>just that you download [[this]](https://github.com/herrlamatv/useful/releases/download/ytdlp.simple.files.1/dw-best.bat)
- and start it.

## If You get Blocked (YouTube)

If you get blocked, you need to verify that you are a human<br>How do I do that? <br>-> You have to export your cookies and work with them. <br> <br>What you need: 

- Firefox (or any browser where you can export your cookies <br>**Chrome** doesnt work.<br>Chrome implemented a protection for Cookie-stealers (malware)
- a Cookie exporting [**extention like this one (click!)**](https://addons.mozilla.org/de/firefox/addon/get-cookies-txt-locally/) ([sourcecode](https://github.com/kairi003/Get-cookies.txt-LOCALLY))
- Idk if you need to but if you do, i'll work for sure: log into **YouTube**
- If you want to be sure: click on a video (random, just to have some activity)

Clicking on the extention shall look like this:
<img width="419" height="124" alt="image" src="https://github.com/user-attachments/assets/e6c7e021-9fa3-4c7c-8b3a-ff0756dd6e03" />

- At that screen you can select **export all cookies** or **export**
- exporting all cookies will give you a "**cookies.txt**"<br>Its the simplest, because its the same across all sites.<br>If you only want to export the cookies of a specific site,<br>you will have to change the "cookies.txt" in the [cook-dw-1080p.bat]()<br> to your .txt file.
- Put the file ("**cookies.txt**") into the directory where the [cook-dw-1080p.bat]() and the [yt-dlp.exe](https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe) are.
- Then start the file and paste the link in (like on the simple thing)
- **THIS IS NOT PERMANENT!**<br>After 10-15 mins, your cookie (yt) will be expired and you will have to regenerate it.


****
### Explatations
**cookie-stealer** : A cookie stealer is a malware ("Virus") that you install on your pc and checks Browsers to export the cookies (exactly what we are doing, just without your perm). <br> **a cookie** : a file/written text that saves your logins, so basicly, if you are logged in on youtube and you delete your cookie: Youtube thinks you are a new person (They know who you are anyways, because there are other mechanics as a tracking pixel, your ip/mac adress & more). <br>The cookie is there, so if you close the browser and reopen it and visit youtube.com, youtube sees there is a cookie created from youtube and you are **automaticly** logged in. <br>What Chrome did: They encrypted it so you/malware cant export it. <br>**Note: DONT SHARE YOUR COOKIE[s] WITH ANYONE!!!**<br> If you want to read smth abt it: [[here]](https://www.malwarebytes.com/cybersecurity/basics/cookie-hijacking)