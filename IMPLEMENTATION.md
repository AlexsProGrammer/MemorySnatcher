
### Stuff need to Fix:


make the missing file donwlaod section load faster, currently it takes quite long so the list will load


thumbnails are not reloaded in the viewer since update to be more consistent with the new data structure. here need update the viewer to reload the thumbnail when the session is loaded, here it needs trigger to reload the cahched data of the viewer when a session is started or stopped from downloader/extractor



when i restart the app the last session will start in run state. here the button are all in state like its running and status also showing running. here i checked nothing is actually running, no process or funciton. here it seems only visual that the state is wrong after restarting the app. here check and fix this.



next please fix a small bug when i reset data while running the extractor, here it fails and shows error msg.
	also sometimes it not fully reset, logs, status. also seems that depsite stopped a mp4 was still running in bg, which maybe failed the reset data through settings. here also fully shutdown the downloader/extractor scripts function when reset data




pelase update the logs on the downlaoder/extractor page. use timestamp for current time, eg [10:19:40]. than state if its a import img or video and also dispaly its date, format (vid/img) and mid (id of the memory). here update all logs to be better for user to dispaly information. also maybe colored eg for error red; or issues/skip, missing yellow

