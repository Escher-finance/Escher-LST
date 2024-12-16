import React, { useState } from "react";
import * as base64js from "base64-js";
import {
  Card,
  CardBody,
  Button,
} from "@nextui-org/react";

import { useGlobalContext } from "@/app/core/context";

const ContractUpload = () => {
  const { userAddress, client } = useGlobalContext();

  const [isUploading, setIsUploading] = useState(false);
  const [fileData, setFileData] = useState<Uint8Array>();
  const [codeID, setCodeID] = useState("");

  const onImageChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    if (event.target.files && event.target.files[0]) {
      let file = event.target.files[0];
      console.log(file);
      readAndUpload(file);
    }
  };

  function readAndUpload(file: File) {
    var reader = new FileReader();
    reader.readAsBinaryString(file);
    reader.onload = function () {
      const resultData = reader?.result?.toString();
      if (resultData) {
        const encoded = Buffer.from(resultData, "binary").toString("base64");
        const contractData = base64js.toByteArray(encoded);
        console.log(contractData);
        setFileData(contractData);
      }
    };
    reader.onerror = function (error) {
      console.log("Error: ", error);
    };
  }

  const upload = async () => {
    console.log("user Address", userAddress);
    const uploadResult = await client.upload(userAddress, fileData, "auto", "");

    console.log("uploadResult");
    const codeId = uploadResult.codeId;
    alert("Code ID:" + codeId);
    return codeId;
  };

  const handleButtonClick = async () => {
    setIsUploading(true);
    try {
      const theCodeID = await upload();

      setCodeID(theCodeID.toString());
    } catch (err) {
      console.log("Failed to upload");
      console.log(err);
    }

    setIsUploading(false);
  };

  return (
    <div className="font-sans text-center mx-auto">
      <Card>
        <CardBody className="gap-4">
          <input type="file" name="myImage" onChange={onImageChange} />
          <Button
            type="submit"
            onClick={handleButtonClick}
            disabled={isUploading}
          >
            {isUploading ? "Uploading ..." : "Upload"}
          </Button>
          {codeID ? <div>Code ID: {codeID}</div> : ""}
        </CardBody>
      </Card>
    </div>
  );
};

export default ContractUpload;
